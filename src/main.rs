use colored::*;

pub mod args;
pub mod rewrite;
pub mod utils;

use crate::rewrite::rewrite_range::rewrite_range_commits;
use crate::rewrite::rewrite_specific::rewrite_specific_commits;
use crate::utils::datetime::generate_timestamps;
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

    args.ensure_all_args_present()?;
    args.validate_simulation_args()?;
    validate_inputs(&args)?;

    match determine_operation_mode(&args) {
        OperationMode::Range => execute_range_operation(&args),
        OperationMode::PickSpecific => execute_pick_specific_operation(&args),
        OperationMode::ShowHistory => execute_show_history_operation(&args),
        OperationMode::FullRewrite => execute_full_rewrite_operation(&mut args),
        OperationMode::Simulate => execute_simulation_operation(&mut args),
    }?;

    if !args.simulate {
        println!("{}", "Operation completed successfully!".green().bold());
    }
    Ok(())
}

#[derive(Debug)]
enum OperationMode {
    Range,
    PickSpecific,
    ShowHistory,
    FullRewrite,
    Simulate,
}

fn determine_operation_mode(args: &Args) -> OperationMode {
    if args.simulate {
        OperationMode::Simulate
    } else if args.range {
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

fn execute_simulation_operation(args: &mut Args) -> Result<()> {
    use crate::utils::commit_history::get_commit_history;
    use crate::utils::simulation::{create_full_rewrite_simulation, print_detailed_diff};

    println!("{}", "🔍 SIMULATION MODE".bold().cyan());
    println!("{}", "Analyzing repository to preview changes...".cyan());

    let commits = get_commit_history(args, false)?;

    if commits.is_empty() {
        println!("{}", "No commits found in repository.".yellow());
        return Ok(());
    }

    // Determine what kind of simulation we can perform based on available arguments
    let simulation_result = if args.range {
        // Range simulation - show that no changes would be made without proper setup
        use crate::utils::simulation::create_specific_commit_simulation;
        create_specific_commit_simulation(&commits, 0, None, None, None, None)?
    } else if args.pick_specific_commits {
        // Pick specific simulation - show that no changes would be made
        use crate::utils::simulation::create_specific_commit_simulation;
        create_specific_commit_simulation(&commits, 0, None, None, None, None)?
    } else {
        // Full rewrite simulation - check if we have the required arguments
        if args.email.is_some() && args.name.is_some() && args.start.is_some() && args.end.is_some()
        {
            // We have all required arguments, do full simulation
            let timestamps = generate_timestamps(args)?;
            create_full_rewrite_simulation(&commits, &timestamps, args)?
        } else {
            // Missing required arguments - show what's needed
            println!(
                "{}",
                "\n⚠️  Incomplete arguments for full simulation."
                    .yellow()
                    .bold()
            );

            let missing = vec![
                if args.name.is_none() {
                    Some("--name")
                } else {
                    None
                },
                if args.email.is_none() {
                    Some("--email")
                } else {
                    None
                },
                if args.start.is_none() {
                    Some("--begin")
                } else {
                    None
                },
                if args.end.is_none() {
                    Some("--end")
                } else {
                    None
                },
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

            if !missing.is_empty() {
                println!(
                    "{} {}",
                    "Missing required arguments:".red(),
                    missing.join(", ").yellow()
                );
                println!("{}", "\nExample usage:".bold());
                println!(
                    "{}",
                    "git-editor --simulate --name \"Your Name\" --email \"your@email.com\" \\"
                        .cyan()
                );
                println!(
                    "{}",
                    "    --begin \"2023-01-01 09:00:00\" --end \"2023-12-31 17:00:00\"".cyan()
                );
                println!();
            }

            // Still show basic repository info
            use crate::utils::simulation::{SimulationResult, SimulationStats};
            let stats = SimulationStats::new(&commits);
            let result = SimulationResult {
                changes: vec![],
                stats,
                operation_mode: "Repository Analysis".to_string(),
            };

            result.stats.print_summary(&result.operation_mode);
            return Ok(());
        }
    };

    // Print summary statistics
    simulation_result
        .stats
        .print_summary(&simulation_result.operation_mode);

    // Print detailed diff if requested
    if args.show_diff {
        print_detailed_diff(&simulation_result);
    }

    Ok(())
}
