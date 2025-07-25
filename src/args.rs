use clap::Parser;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    #[arg(long, help = "Path or URI to the repository")]
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
    pub pic_specific_commits: bool,
}

impl Args {
    pub fn ensure_all_args_present(&mut self) {
        use crate::utils::prompt::prompt_for_missing_arg;

        if self.repo_path.is_none() {
            self.repo_path = Some(prompt_for_missing_arg("repository path"));
        }

        // Skip prompting for email, name, start, and end if using show_history or pic_specific_commits
        if self.show_history || self.pic_specific_commits {
            return;
        }

        if self.email.is_none() {
            self.email = Some(prompt_for_missing_arg("email"));
        }

        if self.name.is_none() {
            self.name = Some(prompt_for_missing_arg("name"));
        }

        if self.start.is_none() {
            self.start = Some(prompt_for_missing_arg("start date (YYYY-MM-DD HH:MM:SS)"));
        }

        if self.end.is_none() {
            self.end = Some(prompt_for_missing_arg("end date (YYYY-MM-DD HH:MM:SS)"));
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
            pic_specific_commits: false,
        };

        assert_eq!(args.repo_path, None);
        assert_eq!(args.email, None);
        assert_eq!(args.name, None);
        assert_eq!(args.start, None);
        assert_eq!(args.end, None);
        assert!(!args.show_history);
        assert!(!args.pic_specific_commits);
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
            pic_specific_commits: false,
        };

        assert_eq!(args.repo_path, Some("/test/repo".to_string()));
        assert!(args.show_history);
        assert!(!args.pic_specific_commits);
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
            pic_specific_commits: true,
        };

        assert_eq!(args.repo_path, Some("/test/repo".to_string()));
        assert!(!args.show_history);
        assert!(args.pic_specific_commits);
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
            pic_specific_commits: false,
        };

        assert_eq!(args.repo_path, Some("/test/repo".to_string()));
        assert_eq!(args.email, Some("test@example.com".to_string()));
        assert_eq!(args.name, Some("Test User".to_string()));
        assert_eq!(args.start, Some("2023-01-01 00:00:00".to_string()));
        assert_eq!(args.end, Some("2023-01-02 00:00:00".to_string()));
    }
}
