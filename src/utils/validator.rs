use crate::args::Args;
use crate::utils::types::Result;
use colored::*;
use regex::Regex;
use std::process;
use url::Url;

pub fn validate_inputs(args: &Args) -> Result<()> {
    // Always validate repo_path as it's always required
    let repo_path = args.repo_path.as_ref().unwrap();

    if repo_path.is_empty() {
        eprintln!("Repository path cannot be empty");
        process::exit(1);
    }
    if Url::parse(repo_path).is_err() && !std::path::Path::new(repo_path).exists() {
        eprintln!(
            "{} {}",
            "Invalid repository path or URL".red().bold(),
            repo_path.yellow()
        );
        process::exit(1);
    }
    if std::path::Path::new(repo_path).exists() {
        if !std::path::Path::new(repo_path).is_dir() {
            eprintln!("Repository path is not a directory {repo_path}");
            process::exit(1);
        }
        if !std::path::Path::new(repo_path).join(".git").exists() {
            eprintln!("Repository path does not contain a valid Git repository {repo_path}");
            process::exit(1);
        }
    }

    // Only validate other fields if not in pick-specific-commits mode or show-history mode
    if !args.pic_specific_commits && !args.show_history {
        let email = args.email.as_ref().unwrap();
        let name = args.name.as_ref().unwrap();
        let start = args.start.as_ref().unwrap();
        let end = args.end.as_ref().unwrap();

        let email_re = Regex::new(r"(?i)^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$")?;
        if !email_re.is_match(email) {
            eprintln!("{} {}", "Invalid email format".red().bold(), email.yellow());
            process::exit(1);
        }

        if name.trim().is_empty() {
            eprintln!("{}", "Name cannot be empty".red().bold());
            process::exit(1);
        }

        let start_re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$")?;
        if !start_re.is_match(start) {
            eprintln!(
                "{} {}",
                "Invalid start date format".red().bold(),
                start.yellow()
            );
            process::exit(1);
        }

        let end_re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$")?;
        if !end_re.is_match(end) {
            eprintln!(
                "{} {}",
                "Invalid end date format".red().bold(),
                end.yellow()
            );
            process::exit(1);
        }

        if start >= end {
            eprintln!("Start date must be before end date");
            process::exit(1);
        }
    }

    Ok(())
}
