use crate::rewrite::engine::Plan;
use crate::utils::message_trailers::rewrite_author_trailers;
use crate::utils::types::CommitInfo;
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
                    crate::utils::sanitize::safe(&self.original_author).red(),
                    crate::utils::sanitize::safe(new_author).green()
                ));
            }
        }

        if let Some(ref new_email) = self.new_email {
            if new_email != &self.original_email {
                changes.push(format!(
                    "Email: {} → {}",
                    crate::utils::sanitize::safe(&self.original_email).red(),
                    crate::utils::sanitize::safe(new_email).green()
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
                    crate::utils::sanitize::safe(original_first_line).red(),
                    crate::utils::sanitize::safe(new_first_line).green()
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
        crate::say!("\n{}", "📊 SIMULATION SUMMARY".bold().cyan());
        crate::say!("{}", "=".repeat(50).cyan());

        crate::say!("{}: {}", "Operation Mode".bold(), operation_mode.yellow());
        crate::say!(
            "{}: {}",
            "Total Commits".bold(),
            self.total_commits.to_string().cyan()
        );
        crate::say!(
            "{}: {}",
            "Commits to Change".bold(),
            if self.commits_to_change > 0 {
                self.commits_to_change.to_string().yellow()
            } else {
                self.commits_to_change.to_string().green()
            }
        );

        if self.commits_to_change > 0 {
            crate::say!("\n{}", "Changes Breakdown:".bold());
            if self.authors_changed > 0 {
                crate::say!(
                    "  • {} commits will have author names changed",
                    self.authors_changed.to_string().yellow()
                );
            }
            if self.emails_changed > 0 {
                crate::say!(
                    "  • {} commits will have author emails changed",
                    self.emails_changed.to_string().yellow()
                );
            }
            if self.timestamps_changed > 0 {
                crate::say!(
                    "  • {} commits will have timestamps changed",
                    self.timestamps_changed.to_string().yellow()
                );
            }
            if self.messages_changed > 0 {
                crate::say!(
                    "  • {} commits will have messages changed",
                    self.messages_changed.to_string().yellow()
                );
            }
        }

        if let (Some(start), Some(end)) = (self.date_range_start, self.date_range_end) {
            crate::say!("\n{}", "Date Range:".bold());
            crate::say!(
                "  {} → {}",
                start.format("%Y-%m-%d %H:%M:%S").to_string().blue(),
                end.format("%Y-%m-%d %H:%M:%S").to_string().blue()
            );
        }

        if self.commits_to_change == 0 {
            crate::say!(
                "\n{}",
                "✅ No changes would be made with current parameters."
                    .green()
                    .bold()
            );
        } else {
            crate::say!(
                "\n{}",
                "⚠️  This is a simulation - no actual changes have been made."
                    .yellow()
                    .bold()
            );
            crate::say!(
                "{}",
                "   Run without --simulate to apply these changes.".bright_black()
            );
        }
    }
}

/// Preview exactly what `engine::apply(plan)` would write, commit by commit.
///
/// Changes are matched to commits by id, so the preview cannot drift from the
/// rewrite regardless of list order.
pub fn simulation_from_plan(
    commits: &[CommitInfo],
    plan: &Plan,
    operation_mode: &str,
) -> SimulationResult {
    let changes: Vec<SimulationChange> = commits
        .iter()
        .map(|commit| {
            let edit = plan.edits.get(&commit.oid).cloned().unwrap_or_default();
            let new_author = edit.name.clone();
            let new_email = edit.email.clone();
            let new_timestamp = edit.time.map(|t| {
                chrono::DateTime::from_timestamp(t.seconds, 0)
                    .unwrap_or_default()
                    .naive_utc()
            });
            let identity_changed = new_author.is_some() || new_email.is_some();
            let base = edit
                .message
                .clone()
                .unwrap_or_else(|| commit.message.clone());
            let message = if identity_changed && commit.message_is_utf8 {
                rewrite_author_trailers(
                    &base,
                    &commit.author_name,
                    &commit.author_email,
                    new_author.as_deref().unwrap_or(&commit.author_name),
                    new_email.as_deref().unwrap_or(&commit.author_email),
                )
            } else {
                base
            };
            let new_message = (message != commit.message).then_some(message);

            SimulationChange {
                commit_oid: commit.oid,
                short_hash: commit.short_hash.clone(),
                original_author: commit.author_name.clone(),
                original_email: commit.author_email.clone(),
                original_timestamp: commit.timestamp,
                original_message: commit.message.clone(),
                new_author: new_author.filter(|n| *n != commit.author_name),
                new_email: new_email.filter(|e| *e != commit.author_email),
                new_timestamp: new_timestamp.filter(|t| *t != commit.timestamp),
                new_message,
            }
        })
        .collect();

    let mut stats = SimulationStats::new(commits);
    stats.update_from_changes(&changes);

    SimulationResult {
        changes,
        stats,
        operation_mode: operation_mode.to_string(),
    }
}

/// Show a preview. In `--json` mode a `final_result` preview (dry run) is the
/// JSON document on stdout; otherwise it is human text (on stderr in JSON mode).
pub fn report_simulation(result: &SimulationResult, show_diff: bool, final_result: bool) {
    if final_result && crate::output::json_mode() {
        crate::output::emit(&crate::output::simulation_json(result));
        return;
    }
    result.stats.print_summary(&result.operation_mode);
    if show_diff {
        print_detailed_diff(result);
    }
}

pub fn print_detailed_diff(result: &SimulationResult) {
    crate::say!("\n{}", "📋 DETAILED CHANGE PREVIEW".bold().cyan());
    crate::say!("{}", "=".repeat(70).cyan());

    let changes_to_show: Vec<_> = result.changes.iter().filter(|c| c.has_changes()).collect();

    if changes_to_show.is_empty() {
        crate::say!("{}", "No changes to display.".green());
        return;
    }

    for (i, change) in changes_to_show.iter().enumerate() {
        crate::say!(
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
            crate::say!("   {summary_line}");
        }

        if i < changes_to_show.len() - 1 {
            crate::say!("{}", "─".repeat(50).bright_black());
        }
    }

    crate::say!(
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
    use crate::rewrite::engine::{CommitterMode, Edit, GitTime};
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
            ..Default::default()
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
    fn test_simulation_from_plan_matches_edits_by_commit_id() {
        // Newest first, as listed by get_commit_history.
        let newest = create_test_commit(
            "2222222222222222222222222222222222222222",
            "Old User",
            "old@example.com",
            "2023-01-02 10:00:00",
            "Second commit",
        );
        let oldest = create_test_commit(
            "1111111111111111111111111111111111111111",
            "Old User",
            "old@example.com",
            "2023-01-01 10:00:00",
            "First commit",
        );
        let early =
            NaiveDateTime::parse_from_str("2023-06-01 09:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let late =
            NaiveDateTime::parse_from_str("2023-06-02 09:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let edit = |t: NaiveDateTime| Edit {
            name: Some("New User".into()),
            email: Some("new@example.com".into()),
            time: Some(GitTime::new(t.and_utc().timestamp(), 0)),
            message: None,
        };
        let plan = Plan {
            edits: [(oldest.oid, edit(early)), (newest.oid, edit(late))]
                .into_iter()
                .collect(),
            committer: CommitterMode::MatchAuthor,
        };

        let result = simulation_from_plan(&[newest, oldest], &plan, "Full History Rewrite");

        assert_eq!(result.stats.commits_to_change, 2);
        assert_eq!(result.changes[0].new_timestamp, Some(late));
        assert_eq!(result.changes[1].new_timestamp, Some(early));
        assert_eq!(result.changes[1].new_author.as_deref(), Some("New User"));
    }

    #[test]
    fn test_simulation_from_plan_ignores_unchanged_values() {
        let commit = create_test_commit(
            "1111111111111111111111111111111111111111",
            "Same User",
            "same@example.com",
            "2023-01-01 10:00:00",
            "msg",
        );
        let plan = Plan {
            edits: [(
                commit.oid,
                Edit {
                    name: Some("Same User".into()),
                    email: Some("same@example.com".into()),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
            committer: CommitterMode::MatchAuthor,
        };
        let result = simulation_from_plan(&[commit], &plan, "x");
        assert_eq!(result.stats.commits_to_change, 0);
    }
}
