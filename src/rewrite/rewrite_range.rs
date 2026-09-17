use crate::rewrite::engine::{self, Edit, GitTime, Plan};
use crate::rewrite::report::print_outcome;
use crate::utils::prompt::read_prompted_line;
use crate::utils::types::CommitInfo;
use crate::utils::types::Result;
use crate::{args::Args, utils::commit_history::get_commit_history};
use chrono::NaiveDateTime;
use colored::Colorize;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{self, Clear, ClearType},
    ExecutableCommand,
};
use git2::Repository;
use std::io::{self, Write};

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

/// What the table loop should do after a key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TableAction {
    Continue,
    SaveAndExit,
    Cancel,
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

/// Draws the table on the alternate screen and restores the terminal (raw mode
/// off, original screen back) when dropped — on success, error or panic.
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> Result<Self> {
        io::stdout().execute(terminal::EnterAlternateScreen)?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = io::stdout().execute(terminal::LeaveAlternateScreen);
        let _ = io::stdout().execute(cursor::Show);
    }
}

struct InteractiveTable {
    commits: Vec<CommitEdit>,
    current_row: usize,
    current_col: TableColumn,
    editing: bool,
    edit_buffer: String,
    /// Last validation error, shown below the table (never stored in a field).
    status: Option<String>,
    editable_fields: (bool, bool, bool, bool), // (author_name, author_email, timestamp, message)
}

impl InteractiveTable {
    fn new(
        commits: Vec<CommitInfo>,
        start_idx: usize,
        end_idx: usize,
        editable_fields: (bool, bool, bool, bool),
    ) -> Self {
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
        let starting_col = if editable_fields.0 {
            // author_name
            TableColumn::AuthorName
        } else if editable_fields.1 {
            // author_email
            TableColumn::AuthorEmail
        } else if editable_fields.2 {
            // timestamp
            TableColumn::Timestamp
        } else if editable_fields.3 {
            // message
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
            status: None,
            editable_fields,
        }
    }

    fn draw_table(&self) {
        // Clear screen using crossterm
        let _ = io::stdout().execute(Clear(ClearType::All));
        let _ = io::stdout().execute(cursor::MoveTo(0, 0));

        println!(
            "{}",
            "Interactive Commit Editor - Range Mode".bold().green()
        );

        // Show which fields are editable
        let editable_info = if self.editable_fields == (true, true, true, true) {
            "All fields editable".to_string()
        } else {
            let mut editable = Vec::new();
            if self.editable_fields.0 || self.editable_fields.1 {
                editable.push("Author");
            }
            if self.editable_fields.2 {
                editable.push("Time");
            }
            if self.editable_fields.3 {
                editable.push("Message");
            }
            format!("Editable: {}", editable.join(", "))
        };
        println!("{}", editable_info.cyan());
        println!(
            "{}",
            "Arrow keys/hjkl: move  Enter: edit  Esc: save & exit  q or Ctrl+C: cancel without saving"
                .yellow()
        );
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

        // Draw only the rows that fit on screen, keeping the cursor row visible.
        let screen_rows = terminal::size().map(|(_, h)| h as usize).unwrap_or(24);
        let (first, count) = viewport(
            self.current_row,
            self.commits.len(),
            screen_rows.saturating_sub(TABLE_CHROME_LINES),
        );
        for (row_idx, commit) in self.commits.iter().enumerate().skip(first).take(count) {
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
            let is_current_cell_index =
                is_current_row && matches!(self.current_col, TableColumn::Index);
            let is_current_cell_hash =
                is_current_row && matches!(self.current_col, TableColumn::Hash);
            let is_current_cell_author_name =
                is_current_row && matches!(self.current_col, TableColumn::AuthorName);
            let is_current_cell_author_email =
                is_current_row && matches!(self.current_col, TableColumn::AuthorEmail);
            let is_current_cell_timestamp =
                is_current_row && matches!(self.current_col, TableColumn::Timestamp);
            let is_current_cell_message =
                is_current_row && matches!(self.current_col, TableColumn::Message);

            let index_final = index_str; // Index is never editable, so no brackets
            let hash_final = hash_str; // Hash is never editable, so no brackets

            let author_name_with_mod = if commit.modifications.author_name_changed {
                format!("*{author_name_str}")
            } else {
                author_name_str
            };
            let author_name_final = author_name_with_mod;

            let author_email_with_mod = if commit.modifications.author_email_changed {
                format!("*{author_email_str}")
            } else {
                author_email_str
            };
            let author_email_final = author_email_with_mod;

            let timestamp_with_mod = if commit.modifications.timestamp_changed {
                format!("*{timestamp_str}")
            } else {
                timestamp_str
            };
            let timestamp_final = timestamp_with_mod;

            let message_with_mod = if commit.modifications.message_changed {
                format!("*{message_str}")
            } else {
                message_str
            };
            let message_final = message_with_mod;

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
                    // Current row, not editing - highlight current cell with special background
                    let index_styled = if is_current_cell_index {
                        index_final.white().on_blue()
                    } else {
                        index_final.white().on_bright_black()
                    };
                    let hash_styled = if is_current_cell_hash {
                        hash_final.white().on_blue()
                    } else {
                        hash_final.yellow().on_bright_black()
                    };
                    let author_name_styled =
                        if is_current_cell_author_name && self.editable_fields.0 {
                            author_name_final.white().on_blue()
                        } else {
                            author_name_final.cyan().on_bright_black()
                        };
                    let author_email_styled =
                        if is_current_cell_author_email && self.editable_fields.1 {
                            author_email_final.white().on_blue()
                        } else {
                            author_email_final.blue().on_bright_black()
                        };
                    let timestamp_styled = if is_current_cell_timestamp && self.editable_fields.2 {
                        timestamp_final.white().on_blue()
                    } else {
                        timestamp_final.magenta().on_bright_black()
                    };
                    let message_styled = if is_current_cell_message && self.editable_fields.3 {
                        message_final.white().on_blue()
                    } else {
                        message_final.green().on_bright_black()
                    };

                    println!(
                        "{index_styled:<4} {hash_styled:<8} {author_name_styled:<15} {author_email_styled:<20} {timestamp_styled:<19} {message_styled}"
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

        if count < self.commits.len() {
            println!(
                "{}",
                format!(
                    "Rows {}-{} of {} (scroll with ↑↓)",
                    first + 1,
                    first + count,
                    self.commits.len()
                )
                .dimmed()
            );
        }
        println!();

        if let Some(status) = &self.status {
            println!("{} {}", "Error:".red().bold(), status.red());
        }
        if self.editing {
            println!("{}: {}", "Editing".bold().yellow(), self.edit_buffer);
            println!(
                "{}",
                "Press Enter to save, Esc to cancel this edit, Ctrl+C to abandon all edits"
                    .italic()
            );
        } else {
            println!(
                "{}",
                "Navigation: ←→↑↓  Edit: Enter  Save & Exit: Esc  Cancel: q / Ctrl+C".italic()
            );
            println!(
                "{}",
                "Tip: Use '*' when selecting range to edit ALL commits at once".dimmed()
            );
        }
    }

    fn truncate_text(&self, text: &str, max_width: usize) -> String {
        truncate_chars(text, max_width)
    }

    fn handle_navigation_key_input(&mut self, key: KeyCode) -> TableAction {
        match key {
            KeyCode::Up if self.current_row > 0 => {
                self.current_row -= 1;
            }
            KeyCode::Down if self.current_row < self.commits.len() - 1 => {
                self.current_row += 1;
            }
            KeyCode::Left => {
                self.move_to_prev_editable_column();
            }
            KeyCode::Right => {
                self.move_to_next_editable_column();
            }
            KeyCode::Char('h') => {
                // Left (vim-style)
                self.move_to_prev_editable_column();
            }
            KeyCode::Char('l') => {
                // Right (vim-style)
                self.move_to_next_editable_column();
            }
            KeyCode::Char('k') if self.current_row > 0 => {
                // Up (vim-style)
                self.current_row -= 1;
            }
            KeyCode::Char('j') if self.current_row < self.commits.len() - 1 => {
                // Down (vim-style)
                self.current_row += 1;
            }
            KeyCode::Enter => self.start_editing(),
            KeyCode::Esc => return TableAction::SaveAndExit,
            KeyCode::Char('q') => return TableAction::Cancel,
            _ => {}
        }
        TableAction::Continue
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
        let columns = [
            TableColumn::Index,
            TableColumn::Hash,
            TableColumn::AuthorName,
            TableColumn::AuthorEmail,
            TableColumn::Timestamp,
            TableColumn::Message,
        ];

        let current_index = columns
            .iter()
            .position(|c| std::mem::discriminant(c) == std::mem::discriminant(&self.current_col))
            .unwrap_or(0);

        for i in 1..columns.len() {
            let next_index = (current_index + i) % columns.len();
            let next_col = &columns[next_index];
            if self.is_column_editable(next_col) {
                self.current_col = *next_col;
                return;
            }
        }
    }

    fn move_to_prev_editable_column(&mut self) {
        let columns = [
            TableColumn::Index,
            TableColumn::Hash,
            TableColumn::AuthorName,
            TableColumn::AuthorEmail,
            TableColumn::Timestamp,
            TableColumn::Message,
        ];

        let current_index = columns
            .iter()
            .position(|c| std::mem::discriminant(c) == std::mem::discriminant(&self.current_col))
            .unwrap_or(0);

        for i in 1..columns.len() {
            let prev_index = if current_index >= i {
                current_index - i
            } else {
                columns.len() - (i - current_index)
            };
            let prev_col = &columns[prev_index];
            if self.is_column_editable(prev_col) {
                self.current_col = *prev_col;
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
            TableColumn::Timestamp => self.commits[self.current_row]
                .timestamp
                .format("%Y-%m-%d %H:%M:%S")
                .to_string(),
            TableColumn::Message => {
                // Use the full original message when editing, not the truncated display version
                if self.commits[self.current_row].modifications.message_changed {
                    self.commits[self.current_row].message.clone()
                } else {
                    // Get the full original message from the first line or full message
                    self.commits[self.current_row].original.message.clone()
                }
            }
            _ => String::new(),
        };
    }

    fn handle_edit_key_input(&mut self, key: KeyCode) -> TableAction {
        if key != KeyCode::Enter {
            self.status = None;
        }
        match key {
            KeyCode::Esc => {
                // Esc - cancel edit
                self.editing = false;
                self.edit_buffer.clear();
            }
            KeyCode::Enter => {
                // Enter - save edit
                if let Err(e) = self.save_current_edit() {
                    // Keep the user's input and stay in edit mode; show the error separately.
                    self.status = Some(format!(
                        "{e} (fix the value or press Esc to cancel the edit)"
                    ));
                    return TableAction::Continue;
                }
                self.status = None;
                self.editing = false;
                self.edit_buffer.clear();
            }
            KeyCode::Backspace => {
                self.edit_buffer.pop();
            }
            KeyCode::Char(c) => {
                // Handle printable characters
                self.edit_buffer.push(c);
            }
            _ => {}
        }
        TableAction::Continue
    }

    /// Route one key press; Ctrl+C always abandons the session.
    fn handle_key(&mut self, key: KeyEvent) -> TableAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return TableAction::Cancel;
        }
        if self.editing {
            self.handle_edit_key_input(key.code)
        } else {
            self.handle_navigation_key_input(key.code)
        }
    }

    fn save_current_edit(&mut self) -> Result<()> {
        let commit = &mut self.commits[self.current_row];

        match self.current_col {
            TableColumn::AuthorName => {
                crate::utils::validator::validate_identity_part(&self.edit_buffer, "Author name")?;
                if commit.author_name != self.edit_buffer {
                    commit.author_name = self.edit_buffer.clone();
                    commit.modifications.author_name_changed =
                        commit.original.author_name != commit.author_name;
                }
            }
            TableColumn::AuthorEmail => {
                if self.edit_buffer.trim().is_empty() {
                    return Err("Author email cannot be empty".into());
                }
                if !crate::utils::validator::is_valid_email(&self.edit_buffer) {
                    return Err("Invalid email format".into());
                }
                if commit.author_email != self.edit_buffer {
                    commit.author_email = self.edit_buffer.clone();
                    commit.modifications.author_email_changed =
                        commit.original.author_email != commit.author_email;
                }
            }
            TableColumn::Timestamp => {
                let new_timestamp =
                    NaiveDateTime::parse_from_str(&self.edit_buffer, "%Y-%m-%d %H:%M:%S")
                        .map_err(|_| "Invalid timestamp format (use YYYY-MM-DD HH:MM:SS)")?;

                if commit.timestamp != new_timestamp {
                    commit.timestamp = new_timestamp;
                    commit.modifications.timestamp_changed =
                        commit.original.timestamp != commit.timestamp;
                }
            }
            TableColumn::Message => {
                if self.edit_buffer.trim().is_empty() {
                    return Err("Commit message cannot be empty".into());
                }
                if commit.message != self.edit_buffer {
                    commit.message = self.edit_buffer.clone();
                    commit.modifications.message_changed =
                        commit.original.message != commit.message;
                }
            }
            _ => {}
        }
        // A value edited back to the original no longer counts as a change.
        let m = &commit.modifications;
        commit.is_modified = m.author_name_changed
            || m.author_email_changed
            || m.timestamp_changed
            || m.message_changed;
        Ok(())
    }

    fn run(&mut self) -> Result<bool> {
        let _terminal = TerminalGuard::enter()?;
        loop {
            // Disable raw mode for drawing the table
            let _ = terminal::disable_raw_mode();
            self.draw_table();

            // Enable raw mode only for reading input
            terminal::enable_raw_mode()?;

            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match self.handle_key(key) {
                    TableAction::Continue => {}
                    TableAction::SaveAndExit => break Ok(true),
                    TableAction::Cancel => break Ok(false),
                }
            }
        }
    }

    fn get_modified_commits(&self) -> Vec<&CommitEdit> {
        self.commits.iter().filter(|c| c.is_modified).collect()
    }
}

/// Lines used by the table header, footer and help text.
const TABLE_CHROME_LINES: usize = 12;

/// First visible row and number of rows for a window of `height` rows that keeps `current` visible.
fn viewport(current: usize, total: usize, height: usize) -> (usize, usize) {
    let height = height.max(3).min(total);
    let first = current.saturating_sub(height / 2).min(total - height);
    (first, height)
}

/// Shorten `text` to at most `max_width` characters (never splitting a UTF-8 char).
fn truncate_chars(text: &str, max_width: usize) -> String {
    if text.chars().count() > max_width {
        let kept: String = text.chars().take(max_width.saturating_sub(1)).collect();
        format!("{kept}…")
    } else {
        text.to_string()
    }
}

pub fn parse_range_input(input: &str, total_commits: usize) -> Result<(usize, usize)> {
    let trimmed_input = input.trim();

    // Check if user entered '*' to select all commits
    if trimmed_input == "*" {
        if total_commits == 0 {
            return Err("No commits available to select".into());
        }
        return Ok((1, total_commits)); // Return 1-based indexing for all commits
    }

    let parts: Vec<&str> = trimmed_input.split('-').collect();

    if parts.len() != 2 {
        return Err("Invalid range format. Use format like '5-11' or '*' for all commits".into());
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
        "Enter range in format 'start-end' (e.g., '5-11') or '*' for all commits:"
            .bold()
            .green()
    );
    print!("{} {} ", "Range:".bold(), "(Esc to cancel)".bright_black());
    io::stdout().flush()?;

    let input = read_prompted_line()?;

    let (start, end) = parse_range_input(&input, commits.len())?;

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
    let total_selected = end_idx - start_idx + 1;
    let is_all_commits = total_selected == commits.len();

    if is_all_commits {
        println!("\n{}", "Selected All Commits for Editing:".bold().green());
    } else {
        println!("\n{}", "Selected Commit Range:".bold().green());
    }
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
    if is_all_commits {
        println!(
            "{} {} commits selected for editing {}",
            "Total:".bold(),
            total_selected.to_string().green(),
            "(ALL COMMITS)".bold().yellow()
        );
    } else {
        println!(
            "{} {} commits selected for editing",
            "Total:".bold(),
            total_selected.to_string().green()
        );
    }

    Ok(())
}

pub fn get_range_edit_info(args: &Args) -> Result<(String, String, NaiveDateTime, NaiveDateTime)> {
    println!("\n{}", "Range Edit Configuration:".bold().green());

    // Get author name
    let author_name = if let Some(name) = &args.name {
        name.clone()
    } else {
        print!(
            "{} {} ",
            "New author name:".bold(),
            "(Esc to cancel)".bright_black()
        );
        io::stdout().flush()?;
        read_prompted_line()?
    };

    // Get author email
    let author_email = if let Some(email) = &args.email {
        email.clone()
    } else {
        print!(
            "{} {} ",
            "New author email:".bold(),
            "(Esc to cancel)".bright_black()
        );
        io::stdout().flush()?;
        read_prompted_line()?
    };

    // Get start timestamp
    let start_timestamp = if let Some(start) = &args.start {
        NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid start timestamp format")?
    } else {
        print!(
            "{} {} ",
            "Start timestamp (YYYY-MM-DD HH:MM:SS):".bold(),
            "(Esc to cancel)".bright_black()
        );
        io::stdout().flush()?;
        let input = read_prompted_line()?;
        NaiveDateTime::parse_from_str(&input, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid start timestamp format")?
    };

    // Get end timestamp
    let end_timestamp = if let Some(end) = &args.end {
        NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid end timestamp format")?
    } else {
        print!(
            "{} {} ",
            "End timestamp (YYYY-MM-DD HH:MM:SS):".bold(),
            "(Esc to cancel)".bright_black()
        );
        io::stdout().flush()?;
        let input = read_prompted_line()?;
        NaiveDateTime::parse_from_str(&input, "%Y-%m-%d %H:%M:%S")
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
    // Fail early on a detached/unborn HEAD and remember the tip the edits are based on.
    let head = engine::current_branch(&Repository::open(args.repo_path.as_ref().unwrap())?)?.head;

    let (start_idx, end_idx) = select_commit_range(&commits)?;

    // Show range details for user feedback
    show_range_details(&commits, start_idx, end_idx)?;

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
                commit_edit
                    .original
                    .timestamp
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
                    .red(),
                commit_edit
                    .timestamp
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string()
                    .green()
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

    print!(
        "\n{} {} ",
        "Apply these changes? (y/n):".bold(),
        "(Esc to cancel)".bright_black()
    );
    io::stdout().flush()?;

    let confirm = read_prompted_line()?;

    if confirm.to_lowercase() != "y" {
        println!("{}", "Operation cancelled.".yellow());
        return Ok(());
    }

    // Apply changes
    apply_interactive_range_changes(args, &table.commits, head)?;

    println!("\n{}", "✓ Commit range successfully edited!".green().bold());

    if args.show_history {
        get_commit_history(args, true)?;
    }

    Ok(())
}

/// Rewrite only the commits edited in the table; older commits keep their ids.
fn apply_interactive_range_changes(
    args: &Args,
    edited_commits: &[CommitEdit],
    expected_head: git2::Oid,
) -> Result<()> {
    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;
    let edits = edited_commits
        .iter()
        .filter(|c| c.is_modified)
        .map(|c| {
            let m = &c.modifications;
            let edit = Edit {
                name: m.author_name_changed.then(|| c.author_name.clone()),
                email: m.author_email_changed.then(|| c.author_email.clone()),
                time: m
                    .timestamp_changed
                    .then(|| GitTime::new(c.timestamp.and_utc().timestamp(), 0)),
                message: m.message_changed.then(|| c.message.clone()),
            };
            (c.original.oid, edit)
        })
        .collect();
    let plan = Plan {
        edits,
        committer: args.committer.into(),
    };
    let outcome = engine::apply(&repo, &plan, expected_head)?;
    print_outcome(&outcome);
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

    fn table_with_one_commit() -> InteractiveTable {
        let commit = CommitInfo {
            author_name: "Old Author".into(),
            author_email: "old@example.com".into(),
            message: "msg\n".into(),
            ..Default::default()
        };
        InteractiveTable::new(vec![commit], 0, 0, (true, true, true, true))
    }

    #[test]
    fn test_validation_error_is_not_saved_as_value() {
        let mut table = table_with_one_commit();
        table.current_col = TableColumn::AuthorName;
        table.start_editing();
        for _ in 0.."Old Author".len() {
            table.handle_edit_key_input(KeyCode::Backspace);
        }
        table.handle_edit_key_input(KeyCode::Enter);
        assert!(table.status.is_some());
        assert!(table.editing);
        assert_eq!(table.edit_buffer, "");
        table.handle_edit_key_input(KeyCode::Enter);
        assert_eq!(table.commits[0].author_name, "Old Author");
        assert!(!table.commits[0].is_modified);
    }

    #[test]
    fn test_ctrl_c_and_q_cancel_esc_saves() {
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let mut table = table_with_one_commit();
        assert_eq!(table.handle_key(ctrl_c), TableAction::Cancel);
        table.start_editing();
        assert_eq!(table.handle_key(ctrl_c), TableAction::Cancel);
        assert!(!table.edit_buffer.ends_with('c'));

        let mut table = table_with_one_commit();
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(table.handle_key(q), TableAction::Cancel);
        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(table.handle_key(esc), TableAction::SaveAndExit);
    }

    #[test]
    fn test_reverted_edit_is_not_a_change() {
        let mut table = table_with_one_commit();
        table.current_col = TableColumn::AuthorName;
        table.start_editing();
        table.edit_buffer = "Someone Else".into();
        table.handle_edit_key_input(KeyCode::Enter);
        assert!(table.commits[0].is_modified);
        table.start_editing();
        table.edit_buffer = "Old Author".into();
        table.handle_edit_key_input(KeyCode::Enter);
        assert!(!table.commits[0].is_modified);
        assert!(table.get_modified_commits().is_empty());
    }

    #[test]
    fn test_viewport_keeps_current_row_visible() {
        assert_eq!(viewport(0, 5, 20), (0, 5));
        assert_eq!(viewport(0, 100, 10), (0, 10));
        assert_eq!(viewport(50, 100, 10), (45, 10));
        assert_eq!(viewport(99, 100, 10), (90, 10));
        assert_eq!(viewport(1, 2, 0), (0, 2));
        for current in 0..100 {
            let (first, count) = viewport(current, 100, 7);
            assert!(first <= current && current < first + count);
        }
    }

    #[test]
    fn test_truncate_chars_handles_multibyte_text() {
        assert_eq!(truncate_chars("张三李四王五", 5), "张三李四…");
        assert_eq!(truncate_chars("émoji 🎉 ok", 20), "émoji 🎉 ok");
        assert_eq!(truncate_chars("abcdef", 4), "abc…");
        assert_eq!(truncate_chars("", 3), "");
    }

    #[test]
    fn test_parse_range_input_valid() {
        let result = parse_range_input("5-11", 20);
        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 5);
        assert_eq!(end, 11);
    }

    #[test]
    fn test_parse_range_input_with_spaces() {
        let result = parse_range_input(" 3 - 8 ", 20);
        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 3);
        assert_eq!(end, 8);
    }

    #[test]
    fn test_parse_range_input_asterisk() {
        let result = parse_range_input("*", 10);
        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 1);
        assert_eq!(end, 10);

        // Test with empty repository
        let result = parse_range_input("*", 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_range_input_invalid_format() {
        let result = parse_range_input("5", 20);
        assert!(result.is_err());

        let result = parse_range_input("5-11-15", 20);
        assert!(result.is_err());

        let result = parse_range_input("abc-def", 20);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_range_input_invalid_range() {
        let result = parse_range_input("11-5", 20);
        assert!(result.is_err());

        let result = parse_range_input("0-5", 20);
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
            ..Default::default()
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

    #[test]
    fn test_apply_range_changes_correct_commit_ordering() {
        // This test verifies that editing commit at display index 0 (newest)
        // actually modifies the newest commit, not the oldest.
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path.clone()),
            range: true,
            ..Default::default()
        };

        // get_commit_history returns newest-first
        let commits = get_commit_history(&args, false).unwrap();
        assert_eq!(commits.len(), 5);
        assert_eq!(commits[0].message, "Commit 5"); // newest
        assert_eq!(commits[4].message, "Commit 1"); // oldest

        // Simulate editing only the newest commit (display index 0)
        let new_timestamp =
            NaiveDateTime::parse_from_str("2099-06-15 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        let mut edited_commits: Vec<CommitEdit> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| CommitEdit {
                index: i,
                original: c.clone(),
                author_name: c.author_name.clone(),
                author_email: c.author_email.clone(),
                timestamp: c.timestamp,
                message: c.message.clone(),
                is_modified: false,
                modifications: ModificationFlags::default(),
            })
            .collect();

        // Mark only index 0 (newest = "Commit 5") as modified
        edited_commits[0].timestamp = new_timestamp;
        edited_commits[0].is_modified = true;
        edited_commits[0].modifications.timestamp_changed = true;

        // Apply changes
        let original_ids: Vec<_> = commits.iter().map(|c| c.oid).collect();
        apply_interactive_range_changes(&args, &edited_commits, commits[0].oid).unwrap();

        // Re-read and verify
        let updated_commits = get_commit_history(&args, false).unwrap();
        assert_eq!(updated_commits.len(), 5);

        // The newest commit (index 0, "Commit 5") should have the new timestamp
        assert_eq!(
            updated_commits[0].timestamp, new_timestamp,
            "Newest commit should have the edited timestamp"
        );

        // The oldest commit (index 4, "Commit 1") should NOT have the new timestamp
        assert_ne!(
            updated_commits[4].timestamp, new_timestamp,
            "Oldest commit should NOT have the edited timestamp"
        );

        // All older commits are reused untouched: same ids, same timestamps.
        for (i, commit) in updated_commits.iter().enumerate().skip(1) {
            assert_eq!(commit.oid, original_ids[i], "commit {i} must keep its id");
            assert_eq!(commit.timestamp, commits[i].timestamp);
        }
    }

    #[test]
    fn test_editing_middle_commit_keeps_older_commit_ids() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path.clone()),
            range: true,
            ..Default::default()
        };
        let commits = get_commit_history(&args, false).unwrap();
        let mut edits: Vec<CommitEdit> = commits
            .iter()
            .enumerate()
            .map(|(i, c)| CommitEdit {
                index: i,
                original: c.clone(),
                author_name: c.author_name.clone(),
                author_email: c.author_email.clone(),
                timestamp: c.timestamp,
                message: c.message.clone(),
                is_modified: false,
                modifications: ModificationFlags::default(),
            })
            .collect();
        // Display index 2 = "Commit 3"; commits 1 and 2 are older.
        edits[2].timestamp += chrono::Duration::hours(1);
        edits[2].is_modified = true;
        edits[2].modifications.timestamp_changed = true;

        apply_interactive_range_changes(&args, &edits, commits[0].oid).unwrap();

        let updated = get_commit_history(&args, false).unwrap();
        assert_eq!(updated[3].oid, commits[3].oid);
        assert_eq!(updated[4].oid, commits[4].oid);
        assert_ne!(updated[2].oid, commits[2].oid);
        assert_eq!(updated[2].timestamp, edits[2].timestamp);
        assert_eq!(updated[0].message, "Commit 5");
    }
}
