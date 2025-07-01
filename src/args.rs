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

    #[arg(short = 'b', long = "begin", help = "Start date for the commits in YYYY-MM-DD format")]
    pub start: Option<String>,

    #[arg(short = 'e', long = "end", help = "End date for the commits in YYYY-MM-DD format")]
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
