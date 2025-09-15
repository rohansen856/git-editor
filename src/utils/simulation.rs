use crate::args::Args;
use crate::utils::types::{CommitInfo, Result};
use chrono::NaiveDateTime;
use colored::Colorize;

#[derive(Debug, Clone)]
pub struct SimulationChange {
    pub commit_oid: git2::Oid,
    pub short_hash: String,
    pub original_author: String,
    pub original_email: String,
    pub original_timestamp: NaiveDateTime,
    pub original_message: String,
    pub new_author: Option<String>,
    pub new_email: Option<String>,
    pub new_timestamp: Option<NaiveDateTime>,
    pub new_message: Option<String>,
}

#[derive(Debug)]
pub struct SimulationStats {
    pub total_commits: usize,
    pub commits_to_change: usize,
    pub authors_changed: usize,
    pub emails_changed: usize,
    pub timestamps_changed: usize,
    pub messages_changed: usize,
    pub date_range_start: Option<NaiveDateTime>,
    pub date_range_end: Option<NaiveDateTime>,
}

#[derive(Debug)]
pub struct SimulationResult {
    pub changes: Vec<SimulationChange>,
    pub stats: SimulationStats,
    pub operation_mode: String,
}

impl SimulationChange {
    pub fn has_changes(&self) -> bool {
        self.new_author.is_some()
            || self.new_email.is_some()
            || self.new_timestamp.is_some()
            || self.new_message.is_some()
    }

    pub fn get_change_summary(&self) -> Vec<String> {
        let mut changes = Vec::new();

        if let Some(ref new_author) = self.new_author {
            if new_author != &self.original_author {
                changes.push(format!(
                    "Author: {} → {}",
                    self.original_author.red(),
                    new_author.green()
                ));
            }
        }

        if let Some(ref new_email) = self.new_email {
            if new_email != &self.original_email {
                changes.push(format!(
                    "Email: {} → {}",
                    self.original_email.red(),
                    new_email.green()
                ));
            }
        }

        if let Some(ref new_timestamp) = self.new_timestamp {
            if new_timestamp != &self.original_timestamp {
                changes.push(format!(
                    "Date: {} → {}",
                    self.original_timestamp
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                        .red(),
                    new_timestamp
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                        .green()
                ));
            }
        }

        if let Some(ref new_message) = self.new_message {
            let original_first_line = self.original_message.lines().next().unwrap_or("");
            let new_first_line = new_message.lines().next().unwrap_or("");
            if new_first_line != original_first_line {
                changes.push(format!(
                    "Message: {} → {}",
                    original_first_line.red(),
                    new_first_line.green()
                ));
            }
        }

        changes
    }
}

impl SimulationStats {
    pub fn new(commits: &[CommitInfo]) -> Self {
        let total_commits = commits.len();
        let (date_range_start, date_range_end) = if !commits.is_empty() {
            let mut timestamps: Vec<_> = commits.iter().map(|c| c.timestamp).collect();
            timestamps.sort();
            (timestamps.first().copied(), timestamps.last().copied())
        } else {
            (None, None)
        };

        Self {
            total_commits,
            commits_to_change: 0,
            authors_changed: 0,
            emails_changed: 0,
            timestamps_changed: 0,
            messages_changed: 0,
            date_range_start,
            date_range_end,
        }
    }

    pub fn update_from_changes(&mut self, changes: &[SimulationChange]) {
        self.commits_to_change = changes.iter().filter(|c| c.has_changes()).count();

        for change in changes {
            if change.new_author.is_some() {
                self.authors_changed += 1;
            }
            if change.new_email.is_some() {
                self.emails_changed += 1;
            }
            if change.new_timestamp.is_some() {
                self.timestamps_changed += 1;
            }
            if change.new_message.is_some() {
                self.messages_changed += 1;
            }
        }
    }

    pub fn print_summary(&self, operation_mode: &str) {
        println!("\n{}", "📊 SIMULATION SUMMARY".bold().cyan());
        println!("{}", "=".repeat(50).cyan());

        println!("{}: {}", "Operation Mode".bold(), operation_mode.yellow());
        println!(
            "{}: {}",
            "Total Commits".bold(),
            self.total_commits.to_string().cyan()
        );
        println!(
            "{}: {}",
            "Commits to Change".bold(),
            if self.commits_to_change > 0 {
                self.commits_to_change.to_string().yellow()
            } else {
                self.commits_to_change.to_string().green()
            }
        );

        if self.commits_to_change > 0 {
            println!("\n{}", "Changes Breakdown:".bold());
            if self.authors_changed > 0 {
                println!(
                    "  • {} commits will have author names changed",
                    self.authors_changed.to_string().yellow()
                );
            }
            if self.emails_changed > 0 {
                println!(
                    "  • {} commits will have author emails changed",
                    self.emails_changed.to_string().yellow()
                );
            }
            if self.timestamps_changed > 0 {
                println!(
                    "  • {} commits will have timestamps changed",
                    self.timestamps_changed.to_string().yellow()
                );
            }
            if self.messages_changed > 0 {
                println!(
                    "  • {} commits will have messages changed",
                    self.messages_changed.to_string().yellow()
                );
            }
        }

        if let (Some(start), Some(end)) = (self.date_range_start, self.date_range_end) {
            println!("\n{}", "Date Range:".bold());
            println!(
                "  {} → {}",
                start.format("%Y-%m-%d %H:%M:%S").to_string().blue(),
                end.format("%Y-%m-%d %H:%M:%S").to_string().blue()
            );
        }

        if self.commits_to_change == 0 {
            println!(
                "\n{}",
                "✅ No changes would be made with current parameters."
                    .green()
                    .bold()
            );
        } else {
            println!(
                "\n{}",
                "⚠️  This is a simulation - no actual changes have been made."
                    .yellow()
                    .bold()
            );
            println!(
                "{}",
                "   Run without --simulate to apply these changes.".bright_black()
            );
        }
    }
}

pub fn create_full_rewrite_simulation(
    commits: &[CommitInfo],
    timestamps: &[NaiveDateTime],
    args: &Args,
) -> Result<SimulationResult> {
    let mut changes = Vec::new();
    let new_author = args.name.as_ref().unwrap();
    let new_email = args.email.as_ref().unwrap();

    for (i, commit) in commits.iter().enumerate() {
        let new_timestamp = timestamps.get(i).copied();

        let change = SimulationChange {
            commit_oid: commit.oid,
            short_hash: commit.short_hash.clone(),
            original_author: commit.author_name.clone(),
            original_email: commit.author_email.clone(),
            original_timestamp: commit.timestamp,
            original_message: commit.message.clone(),
            new_author: Some(new_author.clone()),
            new_email: Some(new_email.clone()),
            new_timestamp,
            new_message: None, // Full rewrite doesn't change messages
        };

        changes.push(change);
    }

    let mut stats = SimulationStats::new(commits);
    stats.update_from_changes(&changes);

    Ok(SimulationResult {
        changes,
        stats,
        operation_mode: "Full Repository Rewrite".to_string(),
    })
}

pub fn create_range_simulation(
    commits: &[CommitInfo],
    selected_range: (usize, usize),
    range_timestamps: &[NaiveDateTime],
    args: &Args,
) -> Result<SimulationResult> {
    let mut changes = Vec::new();
    let (start_idx, end_idx) = selected_range;

    for (i, commit) in commits.iter().enumerate() {
        let change = if i >= start_idx && i <= end_idx {
            let timestamp_idx = i - start_idx;
            let new_timestamp = range_timestamps.get(timestamp_idx).copied();

            SimulationChange {
                commit_oid: commit.oid,
                short_hash: commit.short_hash.clone(),
                original_author: commit.author_name.clone(),
                original_email: commit.author_email.clone(),
                original_timestamp: commit.timestamp,
                original_message: commit.message.clone(),
                new_author: args.name.clone(),
                new_email: args.email.clone(),
                new_timestamp,
                new_message: None,
            }
        } else {
            // Commits outside range remain unchanged
            SimulationChange {
                commit_oid: commit.oid,
                short_hash: commit.short_hash.clone(),
                original_author: commit.author_name.clone(),
                original_email: commit.author_email.clone(),
                original_timestamp: commit.timestamp,
                original_message: commit.message.clone(),
                new_author: None,
                new_email: None,
                new_timestamp: None,
                new_message: None,
            }
        };

        changes.push(change);
    }

    let mut stats = SimulationStats::new(commits);
    stats.update_from_changes(&changes);

    Ok(SimulationResult {
        changes,
        stats,
        operation_mode: format!("Range Edit (commits {}-{})", start_idx + 1, end_idx + 1),
    })
}

pub fn create_specific_commit_simulation(
    commits: &[CommitInfo],
    selected_commit_idx: usize,
    new_author: Option<String>,
    new_email: Option<String>,
    new_timestamp: Option<NaiveDateTime>,
    new_message: Option<String>,
) -> Result<SimulationResult> {
    let mut changes = Vec::new();

    for (i, commit) in commits.iter().enumerate() {
        let change = if i == selected_commit_idx {
            SimulationChange {
                commit_oid: commit.oid,
                short_hash: commit.short_hash.clone(),
                original_author: commit.author_name.clone(),
                original_email: commit.author_email.clone(),
                original_timestamp: commit.timestamp,
                original_message: commit.message.clone(),
                new_author: new_author.clone(),
                new_email: new_email.clone(),
                new_timestamp,
                new_message: new_message.clone(),
            }
        } else {
            // Other commits remain unchanged
            SimulationChange {
                commit_oid: commit.oid,
                short_hash: commit.short_hash.clone(),
                original_author: commit.author_name.clone(),
                original_email: commit.author_email.clone(),
                original_timestamp: commit.timestamp,
                original_message: commit.message.clone(),
                new_author: None,
                new_email: None,
                new_timestamp: None,
                new_message: None,
            }
        };

        changes.push(change);
    }

    let mut stats = SimulationStats::new(commits);
    stats.update_from_changes(&changes);

    Ok(SimulationResult {
        changes,
        stats,
        operation_mode: "Specific Commit Edit".to_string(),
    })
}

pub fn print_detailed_diff(result: &SimulationResult) {
    println!("\n{}", "📋 DETAILED CHANGE PREVIEW".bold().cyan());
    println!("{}", "=".repeat(70).cyan());

    let changes_to_show: Vec<_> = result.changes.iter().filter(|c| c.has_changes()).collect();

    if changes_to_show.is_empty() {
        println!("{}", "No changes to display.".green());
        return;
    }

    for (i, change) in changes_to_show.iter().enumerate() {
        println!(
            "\n{} {} {} ({})",
            format!("{}.", i + 1).bold(),
            "Commit".bold(),
            change.short_hash.yellow().bold(),
            change.commit_oid.to_string()[..16]
                .to_string()
                .bright_black()
        );

        let change_summary = change.get_change_summary();
        for summary_line in change_summary {
            println!("   {summary_line}");
        }

        if i < changes_to_show.len() - 1 {
            println!("{}", "─".repeat(50).bright_black());
        }
    }

    println!(
        "\n{}",
        format!(
            "Showing {} changes out of {} total commits",
            changes_to_show.len(),
            result.changes.len()
        )
        .bright_black()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;

    fn create_test_commit(
        oid_str: &str,
        author: &str,
        email: &str,
        timestamp_str: &str,
        message: &str,
    ) -> CommitInfo {
        CommitInfo {
            oid: git2::Oid::from_str(oid_str).unwrap(),
            short_hash: oid_str[..8].to_string(),
            timestamp: NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S").unwrap(),
            author_name: author.to_string(),
            author_email: email.to_string(),
            message: message.to_string(),
            parent_count: 1,
        }
    }

    #[test]
    fn test_simulation_change_has_changes() {
        let commit = create_test_commit(
            "1234567890abcdef1234567890abcdef12345678",
            "Test User",
            "test@example.com",
            "2023-01-01 10:00:00",
            "Test commit",
        );

        let change = SimulationChange {
            commit_oid: commit.oid,
            short_hash: commit.short_hash,
            original_author: commit.author_name,
            original_email: commit.author_email,
            original_timestamp: commit.timestamp,
            original_message: commit.message,
            new_author: Some("New Author".to_string()),
            new_email: None,
            new_timestamp: None,
            new_message: None,
        };

        assert!(change.has_changes());
    }

    #[test]
    fn test_simulation_change_no_changes() {
        let commit = create_test_commit(
            "1234567890abcdef1234567890abcdef12345678",
            "Test User",
            "test@example.com",
            "2023-01-01 10:00:00",
            "Test commit",
        );

        let change = SimulationChange {
            commit_oid: commit.oid,
            short_hash: commit.short_hash,
            original_author: commit.author_name,
            original_email: commit.author_email,
            original_timestamp: commit.timestamp,
            original_message: commit.message,
            new_author: None,
            new_email: None,
            new_timestamp: None,
            new_message: None,
        };

        assert!(!change.has_changes());
    }

    #[test]
    fn test_simulation_stats_creation() {
        let commits = vec![
            create_test_commit(
                "1234567890abcdef1234567890abcdef12345678",
                "User1",
                "user1@example.com",
                "2023-01-01 10:00:00",
                "First commit",
            ),
            create_test_commit(
                "abcdef1234567890abcdef1234567890abcdef12",
                "User2",
                "user2@example.com",
                "2023-01-02 15:30:00",
                "Second commit",
            ),
        ];

        let stats = SimulationStats::new(&commits);

        assert_eq!(stats.total_commits, 2);
        assert_eq!(stats.commits_to_change, 0);
        assert!(stats.date_range_start.is_some());
        assert!(stats.date_range_end.is_some());
    }

    #[test]
    fn test_create_full_rewrite_simulation() {
        let commits = vec![create_test_commit(
            "1234567890abcdef1234567890abcdef12345678",
            "Old User",
            "old@example.com",
            "2023-01-01 10:00:00",
            "First commit",
        )];

        let timestamps =
            vec![
                NaiveDateTime::parse_from_str("2023-06-01 09:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            ];

        let args = Args {
            repo_path: Some("./test".to_string()),
            email: Some("new@example.com".to_string()),
            name: Some("New User".to_string()),
            start: Some("2023-06-01 08:00:00".to_string()),
            end: Some("2023-06-01 18:00:00".to_string()),
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: true,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        let result = create_full_rewrite_simulation(&commits, &timestamps, &args).unwrap();

        assert_eq!(result.changes.len(), 1);
        assert_eq!(result.stats.total_commits, 1);
        assert_eq!(result.stats.commits_to_change, 1);
        assert_eq!(result.operation_mode, "Full Repository Rewrite");

        let change = &result.changes[0];
        assert!(change.has_changes());
        assert_eq!(change.new_author.as_ref().unwrap(), "New User");
        assert_eq!(change.new_email.as_ref().unwrap(), "new@example.com");
    }

    #[test]
    fn test_create_specific_commit_simulation() {
        let commits = vec![
            create_test_commit(
                "1234567890abcdef1234567890abcdef12345678",
                "User1",
                "user1@example.com",
                "2023-01-01 10:00:00",
                "First commit",
            ),
            create_test_commit(
                "abcdef1234567890abcdef1234567890abcdef12",
                "User2",
                "user2@example.com",
                "2023-01-02 15:30:00",
                "Second commit",
            ),
        ];

        let result = create_specific_commit_simulation(
            &commits,
            0, // Edit first commit
            Some("New Author".to_string()),
            Some("new@example.com".to_string()),
            None,
            Some("Updated message".to_string()),
        )
        .unwrap();

        assert_eq!(result.changes.len(), 2);
        assert_eq!(result.stats.commits_to_change, 1);

        // First commit should have changes
        assert!(result.changes[0].has_changes());
        assert_eq!(result.changes[0].new_author.as_ref().unwrap(), "New Author");

        // Second commit should not have changes
        assert!(!result.changes[1].has_changes());
    }
}
