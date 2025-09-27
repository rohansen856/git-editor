use clap::Parser;
use colored::Colorize;
use tempfile::TempDir;

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

    #[clap(skip)]
    pub _temp_dir: Option<TempDir>,
}

impl Args {
    pub fn ensure_all_args_present(&mut self) -> crate::utils::types::Result<()> {
        use crate::utils::git_clone::{clone_repository, get_repo_name_from_url, is_git_url};
        use crate::utils::git_config::{get_git_user_email, get_git_user_name};
        use crate::utils::prompt::{prompt_for_missing_arg, prompt_with_default};

        if self.repo_path.is_none() {
            self.repo_path = Some(String::from("./"));
        }

        // Handle Git URL cloning
        let repo_path = self.repo_path.as_ref().unwrap();
        if is_git_url(repo_path) {
            println!("{}", "🔍 Git URL detected - cloning repository...".cyan());
            let repo_name = get_repo_name_from_url(repo_path);
            println!("{} {}", "Repository:".bold(), repo_name.yellow());

            let temp_dir = clone_repository(repo_path)?;
            // Store the temporary directory path
            self.repo_path = Some(temp_dir.path().to_string_lossy().to_string());

            // Keep the temporary directory alive for the duration of the program
            self._temp_dir = Some(temp_dir);
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

        if self.start.is_none() || self.end.is_none() {
            // Get the repository's commit date range to provide smart defaults
            let date_range = self.get_repository_date_range()?;

            if self.start.is_none() {
                if let Some((ref start_date, _)) = date_range {
                    let input = prompt_with_default(
                        "Start date (YYYY-MM-DD HH:MM:SS, press Enter to keep original timestamps)",
                        start_date,
                    )?;
                    // If user input exactly matches the default, set a special flag
                    if input == *start_date {
                        self.start = Some("KEEP_ORIGINAL".to_string());
                    } else {
                        self.start = Some(input);
                    }
                } else {
                    self.start = Some(prompt_for_missing_arg("start date (YYYY-MM-DD HH:MM:SS)")?);
                }
            }

            if self.end.is_none() {
                if let Some((_, ref end_date)) = date_range {
                    let input = prompt_with_default(
                        "End date (YYYY-MM-DD HH:MM:SS, press Enter to keep original timestamps)",
                        end_date,
                    )?;
                    // If user input exactly matches the default, set a special flag
                    if input == *end_date
                        && self.start.as_ref() == Some(&"KEEP_ORIGINAL".to_string())
                    {
                        self.end = Some("KEEP_ORIGINAL".to_string());
                    } else if self.start.as_ref() == Some(&"KEEP_ORIGINAL".to_string()) {
                        // User changed end date but kept start as default - use actual start date
                        if let Some((ref start_date, _)) = date_range {
                            self.start = Some(start_date.clone());
                        }
                        self.end = Some(input);
                    } else {
                        self.end = Some(input);
                    }
                } else {
                    self.end = Some(prompt_for_missing_arg("end date (YYYY-MM-DD HH:MM:SS)")?);
                }
            }
        }

        Ok(())
    }

    pub fn should_keep_original_timestamps(&self) -> bool {
        self.start.as_ref() == Some(&"KEEP_ORIGINAL".to_string())
            && self.end.as_ref() == Some(&"KEEP_ORIGINAL".to_string())
    }

    fn get_repository_date_range(&self) -> crate::utils::types::Result<Option<(String, String)>> {
        use crate::utils::commit_history::get_commit_history;

        if let Some(repo_path) = &self.repo_path {
            // Create a temporary Args instance for getting commit history
            let temp_args = Args {
                repo_path: Some(repo_path.clone()),
                email: None,
                name: None,
                start: None,
                end: None,
                show_history: true, // Use show_history mode to avoid validation requirements
                pick_specific_commits: false,
                range: false,
                simulate: false,
                show_diff: false,
                edit_message: false,
                edit_author: false,
                edit_time: false,
                _temp_dir: None,
            };

            match get_commit_history(&temp_args, false) {
                Ok(commits) => {
                    if commits.is_empty() {
                        return Ok(None);
                    }

                    // Get the date range from the first (newest) and last (oldest) commits
                    let newest_commit = &commits[0];
                    let oldest_commit = &commits[commits.len() - 1];

                    // Format the dates as strings
                    let start_date = oldest_commit
                        .timestamp
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string();
                    let end_date = newest_commit
                        .timestamp
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string();

                    Ok(Some((start_date, end_date)))
                }
                Err(_) => Ok(None), // If we can't get history, don't provide defaults
            }
        } else {
            Ok(None)
        }
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
            _temp_dir: None,
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
            _temp_dir: None,
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
            _temp_dir: None,
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
            _temp_dir: None,
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
            _temp_dir: None,
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
            _temp_dir: None,
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
            _temp_dir: None,
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
            _temp_dir: None,
        };

        let result = args.validate_simulation_args();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("--show-diff requires --simulate"));
    }
}
