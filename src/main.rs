use colored::*;

pub mod args;
pub mod rewrite;
pub mod utils;

use crate::rewrite::rewrite_range::rewrite_range_commits;
use crate::rewrite::rewrite_specific::rewrite_specific_commits;
use crate::utils::datetime::generate_timestamps;
use crate::utils::help::print_help;
use crate::utils::types::Result;
use crate::utils::validator::validate_inputs;
use args::Args;
use clap::Parser;
use rewrite::rewrite_all::rewrite_all_commits;

fn main() -> Result<()> {
    run().unwrap_or_else(|error| {
        eprintln!("{} {}", "Error:".red().bold(), error.to_string().red());
        std::process::exit(1);
    });
    Ok(())
}

fn run() -> Result<()> {
    let mut args = Args::parse();

    // Check if this is a help request (no meaningful arguments provided)
    if args.is_help_request() {
        print_help();
        return Ok(());
    }

    args.ensure_all_args_present()?;
    validate_inputs(&args)?;

    match determine_operation_mode(&args) {
        OperationMode::Range => execute_range_operation(&args),
        OperationMode::PickSpecific => execute_pick_specific_operation(&args),
        OperationMode::ShowHistory => execute_show_history_operation(&args),
        OperationMode::FullRewrite => execute_full_rewrite_operation(&mut args),
    }?;

    println!("{}", "Operation completed successfully!".green().bold());
    Ok(())
}

#[derive(Debug)]
enum OperationMode {
    Range,
    PickSpecific,
    ShowHistory,
    FullRewrite,
}

fn determine_operation_mode(args: &Args) -> OperationMode {
    if args.range {
        OperationMode::Range
    } else if args.pick_specific_commits {
        OperationMode::PickSpecific
    } else if args.show_history {
        OperationMode::ShowHistory
    } else {
        OperationMode::FullRewrite
    }
}

fn execute_range_operation(args: &Args) -> Result<()> {
    println!("{}", "Editing commit range...".cyan());
    rewrite_range_commits(args)
}

fn execute_pick_specific_operation(args: &Args) -> Result<()> {
    println!("{}", "Picking specific commits...".cyan());
    rewrite_specific_commits(args)
}

fn execute_show_history_operation(args: &Args) -> Result<()> {
    println!("{}", "Showing commit history...".cyan());
    use crate::utils::commit_history::get_commit_history;
    get_commit_history(args, true)?;
    Ok(())
}

fn execute_full_rewrite_operation(args: &mut Args) -> Result<()> {
    println!("{}", "Generating timestamps...".cyan());
    let timestamps = generate_timestamps(args)?;

    println!("{}", "Rewriting commits...".cyan());
    rewrite_all_commits(args, timestamps)
}
