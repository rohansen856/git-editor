use crate::args::Args;
use crate::types::Result;
use regex::Regex;
use std::process;

pub fn validate_inputs(args: &Args) -> Result<()> {
    // Access fields with unwrap since we know they're populated after ensure_all_args_present
    let repo_path = args.repo_path.as_ref().unwrap();
    let email = args.email.as_ref().unwrap();
    let name = args.name.as_ref().unwrap();
    let start = args.start.as_ref().unwrap();
    let end = args.end.as_ref().unwrap();

    let email_re = Regex::new(r"(?i)^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$")?;
    if !email_re.is_match(email) {
        eprintln!("Invalid email format: {}", email);
        process::exit(1);
    }

    if name.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }

    let start_re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$")?;
    if !start_re.is_match(start) {
        eprintln!("Invalid start date format: {}", start);
        process::exit(1);
    }

    let end_re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$")?;
    if !end_re.is_match(end) {
        eprintln!("Invalid end date format: {}", end);
        process::exit(1);
    }

    if start >= end {
        eprintln!("Start date must be before end date");
        process::exit(1);
    }

    if repo_path.is_empty() {
        eprintln!("Repository path cannot be empty");
        process::exit(1);
    }
    if !std::path::Path::new(repo_path).exists() {
        eprintln!("Repository path does not exist: {}", repo_path);
        process::exit(1);
    }
    if !std::path::Path::new(repo_path).is_dir() {
        eprintln!("Repository path is not a directory: {}", repo_path);
        process::exit(1);
    }
    if !std::path::Path::new(repo_path).join(".git").exists() {
        eprintln!(
            "Repository path does not contain a valid Git repository: {}",
            repo_path
        );
        process::exit(1);
    }

    Ok(())
}
