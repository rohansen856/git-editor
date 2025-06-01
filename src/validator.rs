use regex::Regex;
use std::process;
use crate::args::Args;
use crate::types::Result;

pub fn validate_inputs(args: &Args) -> Result<()> {
    let email_re = Regex::new(r"(?i)^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$")?;
    if !email_re.is_match(&args.email) {
        eprintln!("Invalid email format: {}", args.email);
        process::exit(1);
    }

    if args.name.trim().is_empty() {
        eprintln!("Name cannot be empty");
        process::exit(1);
    }

    Ok(())
}
