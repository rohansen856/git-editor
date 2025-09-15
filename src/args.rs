use clap::Parser;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    #[arg(
        short = 'r',
        long = "repo-path",
        help = "Path or URI to the repository"
    )]
    pub repo_path: Option<String>,

    #[arg(long, help = "Email associated with the commits")]
    pub email: Option<String>,

    #[arg(short = 'n', long = "name", help = "Name associated with the commits")]
    pub name: Option<String>,

    #[arg(
        short = 'b',
        long = "begin",
        help = "Start date for the commits in YYYY-MM-DD format"
    )]
    pub start: Option<String>,

    #[arg(
        short = 'e',
        long = "end",
        help = "End date for the commits in YYYY-MM-DD format"
    )]
    pub end: Option<String>,

    #[arg(
        short = 's',
        long = "show-history",
        help = "Show updated commit history after rewriting"
    )]
    pub show_history: bool,

    #[arg(
        short = 'p',
        long = "pick-specific-commits",
        help = "Pick specific commits to rewrite. Provide a comma-separated list of commit hashes."
    )]
    pub pick_specific_commits: bool,

    #[arg(
        short = 'x',
        long = "range",
        help = "Edit a range of commits (e.g., --range to interactively select range)"
    )]
    pub range: bool,

    #[arg(
        long = "simulate",
        help = "Show what changes would be made without applying them (dry-run mode)"
    )]
    pub simulate: bool,

    #[arg(
        long = "show-diff",
        help = "Show detailed diff preview in simulation mode (requires --simulate)"
    )]
    pub show_diff: bool,

    #[arg(
        long = "message",
        help = "Edit only commit messages in range mode (-x)"
    )]
    pub edit_message: bool,

    #[arg(
        long = "author",
        help = "Edit only author name and email in range mode (-x)"
    )]
    pub edit_author: bool,

    #[arg(long = "time", help = "Edit only timestamps in range mode (-x)")]
    pub edit_time: bool,
}

impl Args {
    pub fn ensure_all_args_present(&mut self) -> crate::utils::types::Result<()> {
        use crate::utils::git_config::{get_git_user_email, get_git_user_name};
        use crate::utils::prompt::{prompt_for_missing_arg, prompt_with_default};

        if self.repo_path.is_none() {
            self.repo_path = Some(String::from("./"));
        }

        // Skip prompting for email, name, start, and end if using show_history, pick_specific_commits, or simulation modes
        if self.show_history || self.pick_specific_commits || self.simulate {
            return Ok(());
        }

        // Range mode will prompt for its own parameters interactively
        if self.range {
            return Ok(());
        }

        if self.email.is_none() {
            // Try to get email from git config first
            if let Some(git_email) = get_git_user_email() {
                self.email = Some(prompt_with_default("Email", &git_email)?);
            } else {
                self.email = Some(prompt_for_missing_arg("email")?);
            }
        }

        if self.name.is_none() {
            // Try to get name from git config first
            if let Some(git_name) = get_git_user_name() {
                self.name = Some(prompt_with_default("Name", &git_name)?);
            } else {
                self.name = Some(prompt_for_missing_arg("name")?);
            }
        }

        if self.start.is_none() {
            self.start = Some(prompt_for_missing_arg("start date (YYYY-MM-DD HH:MM:SS)")?);
        }

        if self.end.is_none() {
            self.end = Some(prompt_for_missing_arg("end date (YYYY-MM-DD HH:MM:SS)")?);
        }

        Ok(())
    }

    pub fn validate_simulation_args(&self) -> crate::utils::types::Result<()> {
        if self.show_diff && !self.simulate {
            return Err("--show-diff requires --simulate to be enabled".into());
        }
        Ok(())
    }

    pub fn get_editable_fields(&self) -> (bool, bool, bool, bool) {
        // (author_name, author_email, timestamp, message)
        if self.range {
            if self.edit_author || self.edit_time || self.edit_message {
                // Selective editing - only edit specified fields
                let edit_author = self.edit_author;
                let edit_time = self.edit_time;
                let edit_message = self.edit_message;
                (edit_author, edit_author, edit_time, edit_message)
            } else {
                // Default: edit all fields when no specific flags are provided
                (true, true, true, true)
            }
        } else {
            // Not in range mode - this shouldn't be called
            (false, false, false, false)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_default_values() {
        let args = Args {
            repo_path: None,
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        assert_eq!(args.repo_path, None);
        assert_eq!(args.email, None);
        assert_eq!(args.name, None);
        assert_eq!(args.start, None);
        assert_eq!(args.end, None);
        assert!(!args.show_history);
        assert!(!args.pick_specific_commits);
        assert!(!args.range);
    }

    #[test]
    fn test_args_with_show_history() {
        let args = Args {
            repo_path: Some("/test/repo".to_string()),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: true,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        assert_eq!(args.repo_path, Some("/test/repo".to_string()));
        assert!(args.show_history);
        assert!(!args.pick_specific_commits);
    }

    #[test]
    fn test_args_with_pick_specific_commits() {
        let args = Args {
            repo_path: Some("/test/repo".to_string()),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: true,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        assert_eq!(args.repo_path, Some("/test/repo".to_string()));
        assert!(!args.show_history);
        assert!(args.pick_specific_commits);
    }

    #[test]
    fn test_args_full_rewrite() {
        let args = Args {
            repo_path: Some("/test/repo".to_string()),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-02 00:00:00".to_string()),
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        assert_eq!(args.repo_path, Some("/test/repo".to_string()));
        assert_eq!(args.email, Some("test@example.com".to_string()));
        assert_eq!(args.name, Some("Test User".to_string()));
        assert_eq!(args.start, Some("2023-01-01 00:00:00".to_string()));
        assert_eq!(args.end, Some("2023-01-02 00:00:00".to_string()));
    }

    #[test]
    fn test_args_with_range() {
        let args = Args {
            repo_path: Some("/test/repo".to_string()),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: false,
            range: true,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        assert_eq!(args.repo_path, Some("/test/repo".to_string()));
        assert!(!args.show_history);
        assert!(!args.pick_specific_commits);
        assert!(args.range);
    }

    #[test]
    fn test_args_with_simulate() {
        let args = Args {
            repo_path: Some("/test/repo".to_string()),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: true,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        assert!(args.simulate);
        assert!(!args.show_diff);
    }

    #[test]
    fn test_validate_simulation_args_valid() {
        let args = Args {
            repo_path: Some("/test/repo".to_string()),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: true,
            show_diff: true,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        let result = args.validate_simulation_args();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_simulation_args_invalid() {
        let args = Args {
            repo_path: Some("/test/repo".to_string()),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: true,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        let result = args.validate_simulation_args();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("--show-diff requires --simulate"));
    }
}
