use colored::*;

pub mod args;
pub mod rewrite;
pub mod utils;

use crate::rewrite::rewrite_range::rewrite_range_commits;
use crate::rewrite::rewrite_specific::rewrite_specific_commits;
use crate::utils::datetime::generate_timestamps;
use crate::utils::types::Result;
use crate::utils::validator::validate_inputs;
use crate::utils::help::print_help;
use args::Args;
use clap::Parser;
use rewrite::rewrite_all::rewrite_all_commits;

fn main() -> Result<()> {
    let mut args = Args::parse();

    // Check if this is a help request (no meaningful arguments provided)
    if args.is_help_request() {
        print_help();
        return Ok(());
    }

    args.ensure_all_args_present();

    if let Err(e) = validate_inputs(&args) {
        eprintln!(
            "{} {}",
            "Validation error:".red().bold(),
            e.to_string().red()
        );
        return Err(e);
    }

    if args.range {
        println!("{}", "Editing commit range...".cyan());
        rewrite_range_commits(&args)?;
    } else if args.pic_specific_commits {
        println!("{}", "Picking specific commits...".cyan());
        rewrite_specific_commits(&args)?;
    } else if args.show_history {
        println!("{}", "Showing commit history...".cyan());
        use crate::utils::commit_history::get_commit_history;
        get_commit_history(&args, true)?;
    } else {
        println!("{}", "Generating timestamps...".cyan());
        let timestamps = generate_timestamps(&mut args)?;

        println!("{}", "Rewriting commits...".cyan());
        rewrite_all_commits(&args, timestamps)?;
    }

    println!("{}", "Operation completed successfully!".green().bold());
    Ok(())
}
