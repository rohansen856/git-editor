use colored::*;

mod args;
mod rewrite;
mod utils;

use crate::rewrite::rewrite_specific::rewrite_specific_commits;
use crate::utils::datetime::generate_timestamps;
use crate::utils::types::Result;
use crate::utils::validator::validate_inputs;
use args::Args;
use clap::Parser;
use rewrite::rewrite_all::rewrite_all_commits;

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
    if args.pic_specific_commits {
        rewrite_specific_commits(&args)?;
    } else {
        rewrite_all_commits(&args, timestamps)?;
    }

    println!("{}", "Operation completed successfully!".green().bold());
    Ok(())
}
