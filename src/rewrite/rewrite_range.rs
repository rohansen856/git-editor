use crate::utils::types::CommitInfo;
use crate::utils::types::Result;
use crate::{args::Args, utils::commit_history::get_commit_history};
use chrono::NaiveDateTime;
use colored::Colorize;
use git2::{Repository, Signature, Sort, Time};
use std::collections::HashMap;
use std::io::{self, Write, Read};
use std::os::unix::io::AsRawFd;

#[derive(Debug, Clone)]
struct CommitEdit {
    index: usize,
    original: CommitInfo,
    author_name: String,
    author_email: String,
    timestamp: NaiveDateTime,
    message: String,
    is_modified: bool,
    modifications: ModificationFlags,
}

#[derive(Debug, Clone, Default)]
struct ModificationFlags {
    author_name_changed: bool,
    author_email_changed: bool,
    timestamp_changed: bool,
    message_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum TableColumn {
    Index = 0,
    Hash = 1,
    AuthorName = 2,
    AuthorEmail = 3,
    Timestamp = 4,
    Message = 5,
}

struct InteractiveTable {
    commits: Vec<CommitEdit>,
    current_row: usize,
    current_col: TableColumn,
    editing: bool,
    edit_buffer: String,
    original_termios: libc::termios,
    escape_sequence_buffer: Vec<u8>,
    editable_fields: (bool, bool, bool, bool), // (author_name, author_email, timestamp, message)
}

impl InteractiveTable {
    fn new(commits: Vec<CommitInfo>, start_idx: usize, end_idx: usize, editable_fields: (bool, bool, bool, bool)) -> Self {
        let mut commit_edits = Vec::new();
        
        for (i, commit) in commits[start_idx..=end_idx].iter().enumerate() {
            commit_edits.push(CommitEdit {
                index: start_idx + i,
                original: commit.clone(),
                author_name: commit.author_name.clone(),
                author_email: commit.author_email.clone(),
                timestamp: commit.timestamp,
                message: commit.message.clone(), // Keep full message, truncate only for display
                is_modified: false,
                modifications: ModificationFlags::default(),
            });
        }

        // Find the first editable column as starting position
        let starting_col = if editable_fields.0 { // author_name
            TableColumn::AuthorName
        } else if editable_fields.1 { // author_email
            TableColumn::AuthorEmail
        } else if editable_fields.2 { // timestamp
            TableColumn::Timestamp
        } else if editable_fields.3 { // message
            TableColumn::Message
        } else {
            TableColumn::AuthorName // fallback
        };

        Self {
            commits: commit_edits,
            current_row: 0,
            current_col: starting_col,
            editing: false,
            edit_buffer: String::new(),
            original_termios: unsafe { std::mem::zeroed() },
            escape_sequence_buffer: Vec::new(),
            editable_fields,
        }
    }

    fn draw_table(&self) {
        // Clear screen
        print!("\x1B[2J\x1B[H");
        
        println!("{}", "Interactive Commit Editor - Range Mode".bold().green());
        
        // Show which fields are editable
        let editable_info = if self.editable_fields == (true, true, true, true) {
            "All fields editable".to_string()
        } else {
            let mut editable = Vec::new();
            if self.editable_fields.0 || self.editable_fields.1 { editable.push("Author"); }
            if self.editable_fields.2 { editable.push("Time"); }
            if self.editable_fields.3 { editable.push("Message"); }
            format!("Editable: {}", editable.join(", "))
        };
        println!("{}", editable_info.cyan());
        println!("{}", "Use Arrow Keys to navigate, Enter to edit, Esc to save & exit, Ctrl+C to cancel".yellow());
        println!();

        // Print header
        println!(
            "{:<4} {:<8} {:<15} {:<20} {:<19} {}",
            "#".bold().white(),
            "HASH".bold().white(),
            "AUTHOR NAME".bold().white(),
            "AUTHOR EMAIL".bold().white(),
            "TIMESTAMP".bold().white(),
            "MESSAGE".bold().white()
        );

        // Draw rows
        for (row_idx, commit) in self.commits.iter().enumerate() {
            let is_current_row = row_idx == self.current_row;
            
            // Prepare content
            let index_str = format!("{}", commit.index + 1);
            let hash_str = self.truncate_text(&commit.original.short_hash, 8);
            let author_name_str = self.truncate_text(&commit.author_name, 15);
            let author_email_str = self.truncate_text(&commit.author_email, 20);
            let timestamp_str = commit.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
            let first_line_message = commit.message.lines().next().unwrap_or("");
            let message_str = self.truncate_text(first_line_message, 40);

            // Add modification indicators and current cell brackets
            let _is_current_cell_index = is_current_row && matches!(self.current_col, TableColumn::Index);
            let _is_current_cell_hash = is_current_row && matches!(self.current_col, TableColumn::Hash);
            let is_current_cell_author_name = is_current_row && matches!(self.current_col, TableColumn::AuthorName);
            let is_current_cell_author_email = is_current_row && matches!(self.current_col, TableColumn::AuthorEmail);
            let is_current_cell_timestamp = is_current_row && matches!(self.current_col, TableColumn::Timestamp);
            let is_current_cell_message = is_current_row && matches!(self.current_col, TableColumn::Message);
            
            let index_final = index_str; // Index is never editable, so no brackets
            let hash_final = hash_str; // Hash is never editable, so no brackets
            
            let author_name_with_mod = if commit.modifications.author_name_changed {
                format!("*{}", author_name_str)
            } else {
                author_name_str
            };
            let author_name_final = if is_current_cell_author_name && !self.editing && self.editable_fields.0 {
                format!("[{}]", author_name_with_mod)
            } else {
                author_name_with_mod
            };
            
            let author_email_with_mod = if commit.modifications.author_email_changed {
                format!("*{}", author_email_str)
            } else {
                author_email_str
            };
            let author_email_final = if is_current_cell_author_email && !self.editing && self.editable_fields.1 {
                format!("[{}]", author_email_with_mod)
            } else {
                author_email_with_mod
            };
            
            let timestamp_with_mod = if commit.modifications.timestamp_changed {
                format!("*{}", timestamp_str)
            } else {
                timestamp_str
            };
            let timestamp_final = if is_current_cell_timestamp && !self.editing && self.editable_fields.2 {
                format!("[{}]", timestamp_with_mod)
            } else {
                timestamp_with_mod
            };
            
            let message_with_mod = if commit.modifications.message_changed {
                format!("*{}", message_str)
            } else {
                message_str
            };
            let message_final = if is_current_cell_message && !self.editing && self.editable_fields.3 {
                format!("[{}]", message_with_mod)
            } else {
                message_with_mod
            };

            // Apply formatting and colors
            if is_current_row {
                if self.editing {
                    println!(
                        "{:<4} {:<8} {:<15} {:<20} {:<19} {}",
                        index_final.black().on_yellow(),
                        hash_final.black().on_yellow(),
                        author_name_final.black().on_yellow(),
                        author_email_final.black().on_yellow(),
                        timestamp_final.black().on_yellow(),
                        message_final.black().on_yellow()
                    );
                } else {
                    println!(
                        "{:<4} {:<8} {:<15} {:<20} {:<19} {}",
                        index_final.white().on_bright_black(),
                        hash_final.yellow().on_bright_black(),
                        author_name_final.cyan().on_bright_black(),
                        author_email_final.blue().on_bright_black(),
                        timestamp_final.magenta().on_bright_black(),
                        message_final.green().on_bright_black()
                    );
                }
            } else {
                println!(
                    "{:<4} {:<8} {:<15} {:<20} {:<19} {}",
                    index_final.white(),
                    hash_final.yellow(),
                    author_name_final.cyan(),
                    author_email_final.blue(),
                    timestamp_final.magenta(),
                    message_final.green()
                );
            }
        }

        println!();
        
        if self.editing {
            println!("{}: {}", "Editing".bold().yellow(), self.edit_buffer);
            println!("{}", "Press Enter to save, Esc to cancel edit".italic());
        } else {
            println!("{}", "Navigation: ←→↑↓  Edit: Enter  Save & Exit: Esc  Cancel: Ctrl+C".italic());
        }
    }

    fn truncate_text(&self, text: &str, max_width: usize) -> String {
        if text.len() > max_width {
            format!("{}…", &text[..max_width.saturating_sub(1)])
        } else {
            text.to_string()
        }
    }


    fn handle_key_input(&mut self, key: u8) -> Result<bool> {
        // Handle escape sequences (arrow keys)
        if key == 27 { // ESC
            self.escape_sequence_buffer.clear();
            self.escape_sequence_buffer.push(key);
            return Ok(true);
        }
        
        // If we're building an escape sequence
        if !self.escape_sequence_buffer.is_empty() {
            self.escape_sequence_buffer.push(key);
            
            // Check for complete arrow key sequences
            if self.escape_sequence_buffer.len() == 3 {
                let sequence = &self.escape_sequence_buffer;
                if sequence[0] == 27 && sequence[1] == 91 { // ESC[
                    match sequence[2] {
                        65 => { // Up arrow
                            if self.current_row > 0 {
                                self.current_row -= 1;
                            }
                        },
                        66 => { // Down arrow
                            if self.current_row < self.commits.len() - 1 {
                                self.current_row += 1;
                            }
                        },
                        68 => { // Left arrow
                            self.move_to_prev_editable_column();
                        },
                        67 => { // Right arrow
                            self.move_to_next_editable_column();
                        },
                        _ => {}
                    }
                }
                self.escape_sequence_buffer.clear();
            } else if self.escape_sequence_buffer.len() == 2 && self.escape_sequence_buffer[1] != 91 {
                // Not an arrow key sequence, handle as escape
                self.escape_sequence_buffer.clear();
                return Ok(false); // Exit on lone ESC
            }
            return Ok(true);
        }
        
        // Regular key handling
        match key {
            b'h' => { // Left (vim-style)
                self.move_to_prev_editable_column();
            },
            b'l' => { // Right (vim-style)
                self.move_to_next_editable_column();
            },
            b'k' => { // Up (vim-style)
                if self.current_row > 0 {
                    self.current_row -= 1;
                }
            },
            b'j' => { // Down (vim-style)
                if self.current_row < self.commits.len() - 1 {
                    self.current_row += 1;
                }
            },
            10 | 13 => { // Enter
                self.start_editing();
                return Ok(true);
            },
            3 => { // Ctrl+C
                return Ok(false); // Signal to exit with cancel
            },
            _ => {}
        }
        Ok(true)
    }

    fn is_column_editable(&self, col: &TableColumn) -> bool {
        match col {
            TableColumn::Index | TableColumn::Hash => false,
            TableColumn::AuthorName => self.editable_fields.0,
            TableColumn::AuthorEmail => self.editable_fields.1,
            TableColumn::Timestamp => self.editable_fields.2,
            TableColumn::Message => self.editable_fields.3,
        }
    }

    fn move_to_next_editable_column(&mut self) {
        let columns = [TableColumn::Index, TableColumn::Hash, TableColumn::AuthorName, 
                      TableColumn::AuthorEmail, TableColumn::Timestamp, TableColumn::Message];
        
        let current_index = columns.iter().position(|c| std::mem::discriminant(c) == std::mem::discriminant(&self.current_col)).unwrap_or(0);
        
        for i in 1..columns.len() {
            let next_index = (current_index + i) % columns.len();
            let next_col = &columns[next_index];
            if self.is_column_editable(next_col) {
                self.current_col = next_col.clone();
                return;
            }
        }
    }

    fn move_to_prev_editable_column(&mut self) {
        let columns = [TableColumn::Index, TableColumn::Hash, TableColumn::AuthorName, 
                      TableColumn::AuthorEmail, TableColumn::Timestamp, TableColumn::Message];
        
        let current_index = columns.iter().position(|c| std::mem::discriminant(c) == std::mem::discriminant(&self.current_col)).unwrap_or(0);
        
        for i in 1..columns.len() {
            let prev_index = if current_index >= i { current_index - i } else { columns.len() - (i - current_index) };
            let prev_col = &columns[prev_index];
            if self.is_column_editable(prev_col) {
                self.current_col = prev_col.clone();
                return;
            }
        }
    }

    fn start_editing(&mut self) {
        if !self.is_column_editable(&self.current_col) {
            return; // This column is not editable
        }

        self.editing = true;
        
        // Initialize edit buffer with current value
        self.edit_buffer = match self.current_col {
            TableColumn::AuthorName => self.commits[self.current_row].author_name.clone(),
            TableColumn::AuthorEmail => self.commits[self.current_row].author_email.clone(),
            TableColumn::Timestamp => self.commits[self.current_row].timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
            TableColumn::Message => {
                // Use the full original message when editing, not the truncated display version
                if self.commits[self.current_row].modifications.message_changed {
                    self.commits[self.current_row].message.clone()
                } else {
                    // Get the full original message from the first line or full message
                    self.commits[self.current_row].original.message.clone()
                }
            },
            _ => String::new(),
        };
    }

    fn handle_edit_input(&mut self, key: u8) -> Result<bool> {
        match key {
            27 => { // Esc - cancel edit
                self.editing = false;
                self.edit_buffer.clear();
            },
            10 | 13 => { // Enter - save edit
                if let Err(e) = self.save_current_edit() {
                    // On error, show message and stay in edit mode
                    self.edit_buffer = format!("Error: {} (Press Esc to cancel)", e);
                    return Ok(true);
                }
                self.editing = false;
                self.edit_buffer.clear();
            },
            127 | 8 => { // Backspace
                self.edit_buffer.pop();
            },
            32..=126 => { // Printable ASCII characters
                self.edit_buffer.push(key as char);
            },
            3 => { // Ctrl+C
                return Ok(false); // Exit without saving
            },
            _ => {}
        }
        Ok(true)
    }

    fn save_current_edit(&mut self) -> Result<()> {
        let commit = &mut self.commits[self.current_row];
        
        match self.current_col {
            TableColumn::AuthorName => {
                if self.edit_buffer.trim().is_empty() {
                    return Err("Author name cannot be empty".into());
                }
                if commit.author_name != self.edit_buffer {
                    commit.author_name = self.edit_buffer.clone();
                    commit.modifications.author_name_changed = commit.original.author_name != commit.author_name;
                    commit.is_modified = true;
                }
            },
            TableColumn::AuthorEmail => {
                if self.edit_buffer.trim().is_empty() {
                    return Err("Author email cannot be empty".into());
                }
                if !self.edit_buffer.contains('@') {
                    return Err("Invalid email format".into());
                }
                if commit.author_email != self.edit_buffer {
                    commit.author_email = self.edit_buffer.clone();
                    commit.modifications.author_email_changed = commit.original.author_email != commit.author_email;
                    commit.is_modified = true;
                }
            },
            TableColumn::Timestamp => {
                let new_timestamp = NaiveDateTime::parse_from_str(&self.edit_buffer, "%Y-%m-%d %H:%M:%S")
                    .map_err(|_| "Invalid timestamp format (use YYYY-MM-DD HH:MM:SS)")?;
                    
                if commit.timestamp != new_timestamp {
                    commit.timestamp = new_timestamp;
                    commit.modifications.timestamp_changed = commit.original.timestamp != commit.timestamp;
                    commit.is_modified = true;
                }
            },
            TableColumn::Message => {
                if self.edit_buffer.trim().is_empty() {
                    return Err("Commit message cannot be empty".into());
                }
                if commit.message != self.edit_buffer {
                    commit.message = self.edit_buffer.clone();
                    commit.modifications.message_changed = commit.original.message != commit.message;
                    commit.is_modified = true;
                }
            },
            _ => {}
        }
        Ok(())
    }

    fn run(&mut self) -> Result<bool> {
        // Set terminal to raw mode
        let stdin = io::stdin();
        let mut stdin_lock = stdin.lock();
        
        // Enable raw mode
        unsafe {
            libc::tcgetattr(stdin.as_raw_fd(), &mut self.original_termios);
            let mut raw = self.original_termios;
            libc::cfmakeraw(&mut raw);
            libc::tcsetattr(stdin.as_raw_fd(), libc::TCSANOW, &raw);
        }

        let result = loop {
            self.draw_table();
            
            let mut buffer = [0; 1];
            if stdin_lock.read_exact(&mut buffer).is_err() {
                break Ok(false);
            }

            let key = buffer[0];
            
            let should_continue = if self.editing {
                match self.handle_edit_input(key) {
                    Ok(cont) => cont,
                    Err(_) => break Ok(false),
                }
            } else {
                match self.handle_key_input(key) {
                    Ok(cont) => cont,
                    Err(_) => break Ok(false),
                }
            };

            if !should_continue {
                break Ok(true); // User wants to save
            }
        };

        self.restore_terminal();
        result
    }

    fn restore_terminal(&self) {
        unsafe {
            libc::tcsetattr(io::stdin().as_raw_fd(), libc::TCSANOW, &self.original_termios);
        }
        print!("\x1B[2J\x1B[H"); // Clear screen
    }

    fn get_modified_commits(&self) -> Vec<&CommitEdit> {
        self.commits.iter().filter(|c| c.is_modified).collect()
    }
}

pub fn parse_range_input(input: &str) -> Result<(usize, usize)> {
    let parts: Vec<&str> = input.trim().split('-').collect();

    if parts.len() != 2 {
        return Err("Invalid range format. Use format like '5-11'".into());
    }

    let start = parts[0]
        .trim()
        .parse::<usize>()
        .map_err(|_| "Invalid start number in range")?;
    let end = parts[1]
        .trim()
        .parse::<usize>()
        .map_err(|_| "Invalid end number in range")?;

    if start < 1 {
        return Err("Start position must be 1 or greater".into());
    }

    if end < start {
        return Err("End position must be greater than or equal to start position".into());
    }

    Ok((start, end))
}

pub fn select_commit_range(commits: &[CommitInfo]) -> Result<(usize, usize)> {
    println!("\n{}", "Commit History:".bold().green());
    println!("{}", "-".repeat(80).cyan());

    for (i, commit) in commits.iter().enumerate() {
        println!(
            "{:3}. {} {} {} {}",
            i + 1,
            commit.short_hash.yellow().bold(),
            commit
                .timestamp
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .blue(),
            commit.author_name.magenta(),
            commit.message.lines().next().unwrap_or("").white()
        );
    }

    println!("{}", "-".repeat(80).cyan());
    println!(
        "\n{}",
        "Enter range in format 'start-end' (e.g., '5-11'):"
            .bold()
            .green()
    );
    print!("{} ", "Range:".bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let (start, end) = parse_range_input(&input)?;

    if start > commits.len() || end > commits.len() {
        return Err(format!(
            "Range out of bounds. Available commits: 1-{}",
            commits.len()
        )
        .into());
    }

    Ok((start - 1, end - 1)) // Convert to 0-based indexing
}

pub fn show_range_details(commits: &[CommitInfo], start_idx: usize, end_idx: usize) -> Result<()> {
    println!("\n{}", "Selected Commit Range:".bold().green());
    println!("{}", "=".repeat(80).cyan());

    for (idx, commit) in commits[start_idx..=end_idx].iter().enumerate() {
        println!(
            "\n{}: {} ({})",
            format!("Commit {}", start_idx + idx + 1).bold(),
            commit.short_hash.yellow(),
            &commit.oid.to_string()[..8]
        );
        println!(
            "{}: {}",
            "Author".bold(),
            format!("{} <{}>", commit.author_name, commit.author_email).magenta()
        );
        println!(
            "{}: {}",
            "Date".bold(),
            commit
                .timestamp
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .blue()
        );
        println!(
            "{}: {}",
            "Message".bold(),
            commit.message.lines().next().unwrap_or("").white()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!(
        "{} {} commits selected for editing",
        "Total:".bold(),
        (end_idx - start_idx + 1).to_string().green()
    );

    Ok(())
}

pub fn get_range_edit_info(args: &Args) -> Result<(String, String, NaiveDateTime, NaiveDateTime)> {
    println!("\n{}", "Range Edit Configuration:".bold().green());

    // Get author name
    let author_name = if let Some(name) = &args.name {
        name.clone()
    } else {
        print!("{} ", "New author name:".bold());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    // Get author email
    let author_email = if let Some(email) = &args.email {
        email.clone()
    } else {
        print!("{} ", "New author email:".bold());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    // Get start timestamp
    let start_timestamp = if let Some(start) = &args.start {
        NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid start timestamp format")?
    } else {
        print!("{} ", "Start timestamp (YYYY-MM-DD HH:MM:SS):".bold());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        NaiveDateTime::parse_from_str(input.trim(), "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid start timestamp format")?
    };

    // Get end timestamp
    let end_timestamp = if let Some(end) = &args.end {
        NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid end timestamp format")?
    } else {
        print!("{} ", "End timestamp (YYYY-MM-DD HH:MM:SS):".bold());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        NaiveDateTime::parse_from_str(input.trim(), "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid end timestamp format")?
    };

    if end_timestamp <= start_timestamp {
        return Err("End timestamp must be after start timestamp".into());
    }

    Ok((author_name, author_email, start_timestamp, end_timestamp))
}

pub fn generate_range_timestamps(
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    count: usize,
) -> Vec<NaiveDateTime> {
    if count == 0 {
        return vec![];
    }

    if count == 1 {
        return vec![start_time];
    }

    let total_duration = end_time.signed_duration_since(start_time);
    let step_duration = total_duration / (count - 1) as i32;

    (0..count)
        .map(|i| start_time + step_duration * i as i32)
        .collect()
}

pub fn rewrite_range_commits(args: &Args) -> Result<()> {
    let commits = get_commit_history(args, false)?;

    if commits.is_empty() {
        println!("{}", "No commits found!".red());
        return Ok(());
    }

    let (start_idx, end_idx) = select_commit_range(&commits)?;
    
    // Get editable fields based on command line flags
    let editable_fields = args.get_editable_fields();
    
    // Launch interactive table editor
    let mut table = InteractiveTable::new(commits.clone(), start_idx, end_idx, editable_fields);
    let should_save = table.run()?;

    if !should_save {
        println!("{}", "Operation cancelled.".yellow());
        return Ok(());
    }

    let modified_commits = table.get_modified_commits();
    
    if modified_commits.is_empty() {
        println!("{}", "No changes made.".yellow());
        return Ok(());
    }

    // Show summary of changes
    println!("\n{}", "Summary of Changes:".bold().green());
    println!("{}", "=".repeat(80).cyan());
    
    for commit_edit in &modified_commits {
        println!(
            "\n{}: {} ({})",
            format!("Commit {}", commit_edit.index + 1).bold(),
            commit_edit.original.short_hash.yellow(),
            &commit_edit.original.oid.to_string()[..8]
        );
        
        if commit_edit.modifications.author_name_changed {
            println!(
                "  {}: {} -> {}",
                "Author Name".bold(),
                commit_edit.original.author_name.red(),
                commit_edit.author_name.green()
            );
        }
        
        if commit_edit.modifications.author_email_changed {
            println!(
                "  {}: {} -> {}",
                "Author Email".bold(),
                commit_edit.original.author_email.red(),
                commit_edit.author_email.green()
            );
        }
        
        if commit_edit.modifications.timestamp_changed {
            println!(
                "  {}: {} -> {}",
                "Timestamp".bold(),
                commit_edit.original.timestamp.format("%Y-%m-%d %H:%M:%S").to_string().red(),
                commit_edit.timestamp.format("%Y-%m-%d %H:%M:%S").to_string().green()
            );
        }
        
        if commit_edit.modifications.message_changed {
            let original_first_line = commit_edit.original.message.lines().next().unwrap_or("");
            let new_first_line = commit_edit.message.lines().next().unwrap_or("");
            println!(
                "  {}: {} -> {}",
                "Message".bold(),
                original_first_line.red(),
                new_first_line.green()
            );
        }
    }

    print!("\n{} (y/n): ", "Apply these changes?".bold());
    io::stdout().flush()?;

    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;

    if confirm.trim().to_lowercase() != "y" {
        println!("{}", "Operation cancelled.".yellow());
        return Ok(());
    }

    // Apply changes
    apply_interactive_range_changes(args, &commits, &table.commits)?;

    println!("\n{}", "✓ Commit range successfully edited!".green().bold());

    if args.show_history {
        get_commit_history(args, true)?;
    }

    Ok(())
}

fn apply_interactive_range_changes(
    args: &Args,
    _original_commits: &[CommitInfo],
    edited_commits: &[CommitEdit],
) -> Result<()> {
    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;
    let head_ref = repo.head()?;
    let branch_name = head_ref
        .shorthand()
        .ok_or("Detached HEAD or invalid branch")?;
    let full_ref = format!("refs/heads/{branch_name}");

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    let mut orig_oids: Vec<_> = revwalk.filter_map(|id| id.ok()).collect();
    orig_oids.reverse();

    // Create a map for quick lookup of edited commits
    let mut edit_map: HashMap<usize, &CommitEdit> = HashMap::new();
    for commit_edit in edited_commits {
        if commit_edit.is_modified {
            edit_map.insert(commit_edit.index, commit_edit);
        }
    }

    let mut new_map: HashMap<git2::Oid, git2::Oid> = HashMap::new();
    let mut last_new_oid = None;

    for (commit_idx, &oid) in orig_oids.iter().enumerate() {
        let orig = repo.find_commit(oid)?;
        let tree = orig.tree()?;

        let new_parents: Result<Vec<_>> = orig
            .parent_ids()
            .map(|pid| {
                let new_pid = *new_map.get(&pid).unwrap_or(&pid);
                repo.find_commit(new_pid).map_err(|e| e.into())
            })
            .collect();

        let new_oid = if let Some(commit_edit) = edit_map.get(&commit_idx) {
            // This commit has been edited - apply changes
            let author_sig = Signature::new(
                &commit_edit.author_name,
                &commit_edit.author_email,
                &Time::new(commit_edit.timestamp.and_utc().timestamp(), 0),
            )?;

            let committer_sig = Signature::new(
                &commit_edit.author_name,
                &commit_edit.author_email,
                &Time::new(commit_edit.timestamp.and_utc().timestamp(), 0),
            )?;

            // Use the edited message or keep the original if not changed
            let message = if commit_edit.modifications.message_changed {
                &commit_edit.message
            } else {
                orig.message().unwrap_or_default()
            };

            repo.commit(
                None,
                &author_sig,
                &committer_sig,
                message,
                &tree,
                &new_parents?.iter().collect::<Vec<_>>(),
            )?
        } else {
            // Keep other commits as-is but update parent references
            let author = orig.author();
            let committer = orig.committer();

            repo.commit(
                None,
                &author,
                &committer,
                orig.message().unwrap_or_default(),
                &tree,
                &new_parents?.iter().collect::<Vec<_>>(),
            )?
        };

        new_map.insert(oid, new_oid);
        last_new_oid = Some(new_oid);
    }

    if let Some(new_head) = last_new_oid {
        repo.reference(&full_ref, new_head, true, "edited commit range interactively")?;
        println!(
            "{} '{}' -> {}",
            "Updated branch".green(),
            branch_name.cyan(),
            new_head.to_string()[..8].to_string().cyan()
        );
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo_with_commits() -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();

        // Initialize git repo
        let repo = git2::Repository::init(&repo_path).unwrap();

        // Create multiple commits
        for i in 1..=5 {
            let file_path = temp_dir.path().join(format!("test{i}.txt"));
            fs::write(&file_path, format!("test content {i}")).unwrap();

            let mut index = repo.index().unwrap();
            index
                .add_path(std::path::Path::new(&format!("test{i}.txt")))
                .unwrap();
            index.write().unwrap();

            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();

            let sig = git2::Signature::new(
                "Test User",
                "test@example.com",
                &git2::Time::new(1234567890 + i as i64 * 3600, 0),
            )
            .unwrap();

            let parents = if i == 1 {
                vec![]
            } else {
                let head = repo.head().unwrap();
                let parent_commit = head.peel_to_commit().unwrap();
                vec![parent_commit]
            };

            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("Commit {i}"),
                &tree,
                &parents.iter().collect::<Vec<_>>(),
            )
            .unwrap();
        }

        (temp_dir, repo_path)
    }

    #[test]
    fn test_parse_range_input_valid() {
        let result = parse_range_input("5-11");
        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 5);
        assert_eq!(end, 11);
    }

    #[test]
    fn test_parse_range_input_with_spaces() {
        let result = parse_range_input(" 3 - 8 ");
        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 3);
        assert_eq!(end, 8);
    }

    #[test]
    fn test_parse_range_input_invalid_format() {
        let result = parse_range_input("5");
        assert!(result.is_err());

        let result = parse_range_input("5-11-15");
        assert!(result.is_err());

        let result = parse_range_input("abc-def");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_range_input_invalid_range() {
        let result = parse_range_input("11-5");
        assert!(result.is_err());

        let result = parse_range_input("0-5");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_range_timestamps() {
        let start =
            NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end =
            NaiveDateTime::parse_from_str("2023-01-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        let timestamps = generate_range_timestamps(start, end, 5);

        assert_eq!(timestamps.len(), 5);
        assert_eq!(timestamps[0], start);
        assert_eq!(timestamps[4], end);

        // Check that timestamps are evenly distributed
        for i in 1..timestamps.len() {
            assert!(timestamps[i] >= timestamps[i - 1]);
        }
    }

    #[test]
    fn test_generate_range_timestamps_edge_cases() {
        let start =
            NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end =
            NaiveDateTime::parse_from_str("2023-01-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        // Zero count
        let timestamps = generate_range_timestamps(start, end, 0);
        assert_eq!(timestamps.len(), 0);

        // Single timestamp
        let timestamps = generate_range_timestamps(start, end, 1);
        assert_eq!(timestamps.len(), 1);
        assert_eq!(timestamps[0], start);
    }

    #[test]
    fn test_rewrite_range_commits_with_repo() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path),
            email: Some("new@example.com".to_string()),
            name: Some("New User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-01 10:00:00".to_string()),
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        // Test that get_commit_history returns commits for this repo
        let commits = get_commit_history(&args, false).unwrap();
        assert_eq!(commits.len(), 5);

        // Test range validation
        let (start, end) = (0, 2); // 0-based indexing
        assert!(start <= end);
        assert!(end < commits.len());

        // Test timestamp generation
        let start_time =
            NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_time =
            NaiveDateTime::parse_from_str("2023-01-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let timestamps = generate_range_timestamps(start_time, end_time, 3);
        assert_eq!(timestamps.len(), 3);
    }
}
