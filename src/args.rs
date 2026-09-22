use clap::{Parser, ValueEnum};
use colored::Colorize;
use tempfile::TempDir;

/// `--committer` policy, mapped onto the engine's [`CommitterMode`].
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CommitterArg {
    #[default]
    MatchAuthor,
    Keep,
}

impl From<CommitterArg> for crate::rewrite::engine::CommitterMode {
    fn from(arg: CommitterArg) -> Self {
        match arg {
            CommitterArg::MatchAuthor => Self::MatchAuthor,
            CommitterArg::Keep => Self::Keep,
        }
    }
}

#[derive(Parser, Default)]
#[command(author, version, about)]
#[command(group(
    clap::ArgGroup::new("mode")
        .args(["show_history", "pick_specific_commits", "range", "docs"])
        .multiple(false)
))]
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
        help = "Start date for the commits in YYYY-MM-DD HH:MM:SS format"
    )]
    pub start: Option<String>,

    #[arg(
        short = 'e',
        long = "end",
        help = "End date for the commits in YYYY-MM-DD HH:MM:SS format"
    )]
    pub end: Option<String>,

    #[arg(
        short = 's',
        long = "show-history",
        help = "Show commit history with statistics (read-only)"
    )]
    pub show_history: bool,

    #[arg(
        short = 'p',
        long = "pick-specific-commits",
        help = "Interactively pick one commit by number and edit its metadata"
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
        conflicts_with_all = ["show_history", "docs"],
        help = "Show what changes would be made without applying them (dry-run mode)"
    )]
    pub simulate: bool,

    #[arg(
        long = "show-diff",
        help = "Show the per-commit change list in previews (simulation and before confirming a rewrite)"
    )]
    pub show_diff: bool,

    #[arg(
        long = "message",
        requires = "range",
        help = "Edit only commit messages in range mode (-x)"
    )]
    pub edit_message: bool,

    #[arg(
        long = "author",
        requires = "range",
        help = "Edit only author name and email in range mode (-x)"
    )]
    pub edit_author: bool,

    #[arg(
        long = "time",
        requires = "range",
        help = "Edit only timestamps in range mode (-x)"
    )]
    pub edit_time: bool,

    #[arg(
        long = "skip-range-check",
        help = "Skip the minimum date range check (allows tightly packed commit timestamps)"
    )]
    pub skip_range_check: bool,

    #[arg(
        long = "committer",
        value_enum,
        default_value_t = CommitterArg::MatchAuthor,
        help = "Committer of rewritten commits: match-author (follow edited author fields) or keep (leave unchanged)"
    )]
    pub committer: CommitterArg,

    #[arg(
        long = "keep-dates",
        help = "Full rewrite: change only author name/email and keep every commit's original date and time zone"
    )]
    pub keep_dates: bool,

    #[arg(
        long = "json",
        help = "Print one JSON document on stdout (history, preview or rewrite result, or the error); progress goes to stderr"
    )]
    pub json: bool,

    #[arg(
        short = 'y',
        long = "yes",
        help = "Answer yes to the final confirmation (for scripts and agents)"
    )]
    pub yes: bool,

    #[arg(
        long = "select",
        value_name = "RANGE",
        requires = "range",
        help = "With -x: commits to edit without the table, e.g. 2-5, 3 or * (1 = newest, as shown by -s)"
    )]
    pub select: Option<String>,

    #[arg(
        long = "commit",
        value_name = "REF",
        requires = "pick_specific_commits",
        help = "With -p: commit to edit without the menu (hash prefix, or 1-based number from -s; 1 = newest)"
    )]
    pub commit: Option<String>,

    #[arg(
        long = "set-date",
        value_name = "DATE",
        requires = "pick_specific_commits",
        help = "With -p --commit: new author date (YYYY-MM-DD HH:MM:SS [+HH:MM], default UTC)"
    )]
    pub set_date: Option<String>,

    #[arg(
        long = "set-message",
        value_name = "TEXT",
        requires = "pick_specific_commits",
        help = "With -p --commit: new commit message"
    )]
    pub set_message: Option<String>,

    #[arg(
        long = "clone-dir",
        value_name = "DIR",
        help = "When --repo-path is a URL, clone into DIR and keep it (required for modes that rewrite history)"
    )]
    pub clone_dir: Option<String>,

    #[arg(
        long = "docs-out",
        value_name = "PATH",
        requires = "docs",
        help = "With --docs: write the HTML documentation to PATH instead of a temporary file"
    )]
    pub docs_out: Option<String>,

    #[arg(
        long = "docs",
        help = "Open comprehensive documentation in the browser"
    )]
    pub docs: bool,

    #[clap(skip)]
    pub _temp_dir: Option<TempDir>,
}

impl Args {
    pub fn ensure_all_args_present(&mut self) -> crate::utils::types::Result<()> {
        use crate::utils::git_clone::{
            clone_repository, clone_repository_to, get_repo_name_from_url, is_git_url,
        };
        use crate::utils::git_config::{get_git_user_email, get_git_user_name};
        use crate::utils::prompt::{prompt_for_missing_arg, prompt_with_default};

        if self.repo_path.is_none() {
            self.repo_path = Some(String::from("./"));
        }

        // Handle Git URL cloning (not needed for --docs).
        let repo_path = self.repo_path.clone().unwrap_or_default();
        if !self.docs && is_git_url(&repo_path) {
            let read_only = self.show_history || self.simulate;
            match self.clone_dir.clone() {
                Some(dir) => {
                    clone_repository_to(&repo_path, std::path::Path::new(&dir))?;
                    self.repo_path = Some(dir);
                }
                None if read_only => {
                    let temp_dir = clone_repository(&repo_path)?;
                    self.repo_path = Some(temp_dir.path().to_string_lossy().to_string());
                    // Keep the temporary directory alive for the duration of the program
                    self._temp_dir = Some(temp_dir);
                }
                None => {
                    return Err(format!(
                        "Refusing to rewrite '{}': the clone would be deleted on exit and the rewrite lost.\n\
                         Pass --clone-dir <DIR> to keep the rewritten clone (then push from it), or clone it yourself and pass the local path.",
                        crate::utils::git_clone::redact_url(&repo_path)
                    )
                    .into());
                }
            }
            crate::say!(
                "{} {}",
                "Repository:".bold(),
                get_repo_name_from_url(&repo_path).yellow()
            );
        }

        // Resolve a path inside a repository (or a bare repository) to the repository itself.
        if let Some(path) = self.repo_path.clone() {
            if std::path::Path::new(&path).exists() {
                if let Ok(repo) = git2::Repository::discover(&path) {
                    let root = repo.workdir().unwrap_or_else(|| repo.path());
                    self.repo_path = Some(root.to_string_lossy().to_string());
                }
            }
        }

        // Skip prompting for email, name, start, and end if using show_history, pick_specific_commits, simulation, or docs modes
        if self.show_history || self.pick_specific_commits || self.simulate || self.docs {
            return Ok(());
        }

        // Range mode will prompt for its own parameters interactively
        if self.range {
            return Ok(());
        }

        if self.email.is_none() {
            // Try to get email from git config first
            if let Some(git_email) = get_git_user_email(self.repo_path.as_deref()) {
                self.email = Some(prompt_with_default("Email", &git_email)?);
            } else {
                self.email = Some(prompt_for_missing_arg("email")?);
            }
        }

        if self.name.is_none() {
            // Try to get name from git config first
            if let Some(git_name) = get_git_user_name(self.repo_path.as_deref()) {
                self.name = Some(prompt_with_default("Name", &git_name)?);
            } else {
                self.name = Some(prompt_for_missing_arg("name")?);
            }
        }

        if !self.keep_dates && (self.start.is_none() || self.end.is_none()) {
            // Offer the repository's own date range as defaults; accepting both
            // keeps every commit's original date (same as --keep-dates).
            let date_range = self.get_repository_date_range()?;

            match date_range {
                Some((first, last)) => {
                    let start = match self.start.clone() {
                        Some(start) => start,
                        None => prompt_with_default(
                            "Start date (YYYY-MM-DD HH:MM:SS UTC, press Enter to keep original dates)",
                            &first,
                        )?,
                    };
                    let end = match self.end.clone() {
                        Some(end) => end,
                        None => prompt_with_default(
                            "End date (YYYY-MM-DD HH:MM:SS UTC, press Enter to keep original dates)",
                            &last,
                        )?,
                    };
                    if self.start.is_none() && self.end.is_none() && start == first && end == last {
                        self.keep_dates = true;
                    } else {
                        self.start = Some(start);
                        self.end = Some(end);
                    }
                }
                None => {
                    if self.start.is_none() {
                        self.start =
                            Some(prompt_for_missing_arg("start date (YYYY-MM-DD HH:MM:SS)")?);
                    }
                    if self.end.is_none() {
                        self.end = Some(prompt_for_missing_arg("end date (YYYY-MM-DD HH:MM:SS)")?);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn should_keep_original_timestamps(&self) -> bool {
        self.keep_dates
    }

    fn get_repository_date_range(&self) -> crate::utils::types::Result<Option<(String, String)>> {
        use crate::utils::commit_history::get_commit_history;

        if let Some(repo_path) = &self.repo_path {
            // Create a temporary Args instance for getting commit history
            let temp_args = Args {
                repo_path: Some(repo_path.clone()),
                show_history: true, // Use show_history mode to avoid validation requirements
                ..Default::default()
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
                Err(e) => {
                    eprintln!(
                        "{} Could not read commit history for date defaults: {e}",
                        "Warning:".yellow()
                    );
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
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
            ..Default::default()
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
            show_history: true,
            ..Default::default()
        };

        assert_eq!(args.repo_path, Some("/test/repo".to_string()));
        assert!(args.show_history);
        assert!(!args.pick_specific_commits);
    }

    #[test]
    fn test_args_with_pick_specific_commits() {
        let args = Args {
            repo_path: Some("/test/repo".to_string()),
            pick_specific_commits: true,
            ..Default::default()
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
            ..Default::default()
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
            range: true,
            ..Default::default()
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
            simulate: true,
            ..Default::default()
        };

        assert!(args.simulate);
        assert!(!args.show_diff);
    }

    #[test]
    fn test_args_with_docs() {
        let args = Args {
            docs: true,
            ..Default::default()
        };

        assert!(args.docs);
        assert!(!args.show_history);
        assert!(!args.simulate);
        assert!(!args.pick_specific_commits);
        assert!(!args.range);
    }

    #[test]
    fn test_docs_mode_skips_validation() {
        let mut args = Args {
            repo_path: None, // This would normally cause validation to fail
            docs: true,
            ..Default::default()
        };

        // This should not fail even though repo_path is None, because docs mode skips validation
        let result = args.ensure_all_args_present();
        assert!(result.is_ok());
    }

    fn init_repo_with_commit(path: &std::path::Path) {
        let repo = git2::Repository::init(path).unwrap();
        let sig = git2::Signature::now("T", "t@example.com").unwrap();
        let tree = repo
            .find_tree(repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();
    }

    #[test]
    fn test_subdirectory_resolves_to_repository_root() {
        let dir = tempfile::TempDir::new().unwrap();
        init_repo_with_commit(dir.path());
        let sub = dir.path().join("nested/deeper");
        std::fs::create_dir_all(&sub).unwrap();
        let mut args = Args {
            repo_path: Some(sub.to_string_lossy().to_string()),
            show_history: true,
            ..Default::default()
        };
        args.ensure_all_args_present().unwrap();
        let resolved = std::fs::canonicalize(args.repo_path.unwrap()).unwrap();
        assert_eq!(resolved, std::fs::canonicalize(dir.path()).unwrap());
    }

    #[test]
    fn test_bare_repository_is_accepted() {
        let dir = tempfile::TempDir::new().unwrap();
        init_repo_with_commit(&dir.path().join("src"));
        let bare = dir.path().join("bare.git");
        git2::build::RepoBuilder::new()
            .bare(true)
            .clone(dir.path().join("src").to_str().unwrap(), &bare)
            .unwrap();
        let mut args = Args {
            repo_path: Some(bare.to_string_lossy().to_string()),
            show_history: true,
            ..Default::default()
        };
        args.ensure_all_args_present().unwrap();
        assert!(crate::utils::validator::validate_inputs(&args).is_ok());
    }
}
