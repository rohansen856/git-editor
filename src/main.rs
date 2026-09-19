use colored::*;

pub mod args;
pub mod docs;
pub mod rewrite;
pub mod utils;

use crate::rewrite::rewrite_range::rewrite_range_commits;
use crate::rewrite::rewrite_specific::rewrite_specific_commits;
use crate::utils::datetime::generate_timestamps;
use crate::utils::types::Result;
use crate::utils::validator::validate_inputs;
use args::Args;
use clap::Parser;

fn main() -> Result<()> {
    run().unwrap_or_else(|error| {
        let code = crate::utils::prompt::exit_code_for_error(error.as_ref());
        if code != 0 {
            eprintln!("{} {}", "Error:".red().bold(), error.to_string().red());
        }
        std::process::exit(code);
    });
    Ok(())
}

fn run() -> Result<()> {
    let mut args = Args::parse();

    args.ensure_all_args_present()?;
    args.validate_simulation_args()?;
    validate_inputs(&args)?;

    match determine_operation_mode(&args) {
        OperationMode::Docs => execute_docs_operation(),
        OperationMode::Range => execute_range_operation(&args),
        OperationMode::PickSpecific => execute_pick_specific_operation(&args),
        OperationMode::ShowHistory => execute_show_history_operation(&args),
        OperationMode::FullRewrite => execute_full_rewrite_operation(&mut args),
        OperationMode::Simulate => execute_simulation_operation(&mut args),
    }?;

    if !args.simulate && !args.docs {
        println!("{}", "Operation completed successfully!".green().bold());
    }
    Ok(())
}

#[derive(Debug)]
enum OperationMode {
    Docs,
    Range,
    PickSpecific,
    ShowHistory,
    FullRewrite,
    Simulate,
}

fn determine_operation_mode(args: &Args) -> OperationMode {
    if args.docs {
        OperationMode::Docs
    } else if args.simulate {
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

fn execute_docs_operation() -> Result<()> {
    crate::docs::execute_docs_operation()
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
    use crate::rewrite::rewrite_all::{apply_plan, begin_offset, full_rewrite_plan};
    use crate::utils::commit_history::get_commit_history;
    use crate::utils::prompt::confirm;
    use crate::utils::simulation::{print_detailed_diff, simulation_from_plan};

    // First, show a summary of what will be changed
    println!("{}", "📊 SUMMARY OF PLANNED CHANGES".bold().cyan());
    println!("{}", "Analyzing repository...".cyan());

    let commits = get_commit_history(args, false)?;
    if commits.is_empty() {
        println!("{}", "No commits found in repository.".yellow());
        return Ok(());
    }
    // The branch tip the preview is based on; the rewrite refuses to run if it moves.
    let repo = git2::Repository::open(args.repo_path.as_ref().unwrap())?;
    let head = crate::rewrite::engine::current_branch(&repo)?.head;

    // Check if user wants to keep original timestamps
    if args.should_keep_original_timestamps() {
        println!("{}", "✅ Keeping original timestamps as requested.".green());

        // No timestamps: every commit keeps its own author date and offset.
        let plan = full_rewrite_plan(
            &repo,
            head,
            args.name.as_ref().unwrap(),
            args.email.as_ref().unwrap(),
            None,
            0,
            args.committer.into(),
        )?;
        let simulation_result = simulation_from_plan(&commits, &plan, "Author Information Update");

        // Show summary
        simulation_result
            .stats
            .print_summary("Author Information Update");
        print_detailed_diff(&simulation_result);

        // Ask for confirmation
        println!(
            "\n{}",
            "⚠️  This operation will rewrite Git history!"
                .yellow()
                .bold()
        );
        println!(
            "{}",
            "Only author information will be changed, timestamps will remain the same.".cyan()
        );
        println!(
            "{}",
            "Make sure you have backed up your repository.".yellow()
        );

        if !confirm("\nDo you want to proceed?", args.yes)? {
            println!("{}", "❌ Operation cancelled by user.".red());
            return Ok(());
        }

        println!(
            "{}",
            "\n🚀 Proceeding with author information update..."
                .green()
                .bold()
        );
        println!("{}", "Updating author information...".cyan());

        apply_plan(&repo, &plan, head)?;
        Ok(())
    } else {
        let timestamps = generate_timestamps(args)?;
        let plan = full_rewrite_plan(
            &repo,
            head,
            args.name.as_ref().unwrap(),
            args.email.as_ref().unwrap(),
            Some(&timestamps),
            begin_offset(args),
            args.committer.into(),
        )?;
        let simulation_result = simulation_from_plan(&commits, &plan, "Full History Rewrite");

        // Show summary
        simulation_result
            .stats
            .print_summary("Full History Rewrite");
        print_detailed_diff(&simulation_result);

        // Ask for confirmation
        println!(
            "\n{}",
            "⚠️  This operation will rewrite Git history permanently!"
                .yellow()
                .bold()
        );
        println!(
            "{}",
            "Make sure you have backed up your repository.".yellow()
        );

        if !confirm("\nDo you want to proceed?", args.yes)? {
            println!("{}", "❌ Operation cancelled by user.".red());
            return Ok(());
        }

        println!("{}", "\n🚀 Proceeding with rewrite...".green().bold());
        println!("{}", "Rewriting commits...".cyan());
        apply_plan(&repo, &plan, head)?;
        Ok(())
    }
}

fn execute_simulation_operation(args: &mut Args) -> Result<()> {
    use crate::rewrite::engine::current_branch;
    use crate::rewrite::rewrite_all::{begin_offset, full_rewrite_plan};
    use crate::utils::commit_history::get_commit_history;
    use crate::utils::simulation::{print_detailed_diff, simulation_from_plan};

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
            let repo = git2::Repository::open(args.repo_path.as_ref().unwrap())?;
            let head = current_branch(&repo)?.head;
            let plan = full_rewrite_plan(
                &repo,
                head,
                args.name.as_ref().unwrap(),
                args.email.as_ref().unwrap(),
                Some(&timestamps),
                begin_offset(args),
                args.committer.into(),
            )?;
            simulation_from_plan(&commits, &plan, "Full Repository Rewrite")
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
