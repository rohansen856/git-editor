use colored::*;

mod args;
mod rewrite;
mod utils;

use crate::utils::datetime::generate_timestamps;
use crate::utils::types::Result;
use crate::utils::validator::validate_inputs;
use args::Args;
use clap::Parser;
use rewrite::rewrite_commits;

fn main() -> Result<()> {
    let mut args = Args::parse();

    args.ensure_all_args_present();

    if let Err(e) = validate_inputs(&args) {
        eprintln!(
            "{} {}",
            "Validation error:".red().bold(),
            e.to_string().red()
        );
        return Err(e);
    }

    println!("{}", "Generating timestamps...".cyan());
    let timestamps = generate_timestamps(&mut args)?;

    println!("{}", "Rewriting commits...".cyan());
    rewrite_commits(&args, timestamps)?;

    println!("{}", "Operation completed successfully!".green().bold());
    Ok(())
}
