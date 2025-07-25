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

    args.ensure_required_args_present();

    if let Err(e) = validate_inputs(&args) {
        eprintln!(
            "{} {}",
            "Validation error:".red().bold(),
            e.to_string().red()
        );
        return Err(e);
    }

    if args.show_history && !args.pick_specific_commits {
        // Just show history without rewriting
        use crate::utils::commit_history::get_commit_history;
        println!("{}", "Displaying commit history...".cyan());
        get_commit_history(&args, true)?;
    } else if args.pick_specific_commits {
        println!("{}", "Rewriting commits...".cyan());
        rewrite_specific_commits(&args)?;
    } else {
        println!("{}", "Rewriting commits...".cyan());
        println!("{}", "Generating timestamps...".cyan());
        let timestamps = generate_timestamps(&mut args)?;
        rewrite_all_commits(&args, timestamps)?;
    }

    println!("{}", "Operation completed successfully!".green().bold());
    Ok(())
}
