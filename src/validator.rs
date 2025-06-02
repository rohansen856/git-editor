use regex::Regex;
use std::process;
use crate::args::Args;
use crate::types::Result;

pub fn validate_inputs(args: &Args) -> Result<()> {
    // Access fields with unwrap since we know they're populated after ensure_all_args_present
    let email = args.email.as_ref().unwrap();
    let name = args.name.as_ref().unwrap();
    
    let email_re = Regex::new(r"(?i)^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$")?;
    if !email_re.is_match(&email) {
        eprintln!("Invalid email format: {}", email);
        process::exit(1);
    }

    if name.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }

    Ok(())
}
