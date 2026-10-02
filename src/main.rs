use colored::*;

use clap::Parser;
use git_editor::args::Args;
use git_editor::rewrite::rewrite_range::rewrite_range_commits;
use git_editor::rewrite::rewrite_specific::rewrite_specific_commits;
use git_editor::utils::datetime::generate_timestamps;
use git_editor::utils::types::Result;
use git_editor::utils::validator::validate_inputs;

fn main() -> Result<()> {
    run().unwrap_or_else(|error| {
        let code = git_editor::utils::prompt::exit_code_for_error(error.as_ref());
        if git_editor::output::json_mode() {
            let message = if git_editor::utils::prompt::is_cancelled(error.as_ref()) {
                "Operation cancelled".to_string()
            } else {
                error.to_string()
            };
            git_editor::output::emit(&git_editor::output::error_json(&message, code));
        } else if !git_editor::utils::prompt::is_cancelled(error.as_ref()) {
            eprintln!("{} {}", "Error:".red().bold(), error.to_string().red());
        }
        std::process::exit(code);
    });
    Ok(())
}

fn run() -> Result<()> {
    let mut args = Args::parse();
    // Plain text when piped (agents, CI, files); NO_COLOR / CLICOLOR_FORCE still apply.
    if !std::io::IsTerminal::is_terminal(&std::io::stdout())
        && std::env::var_os("CLICOLOR_FORCE").is_none()
    {
        colored::control::set_override(false);
    }
    if args.json {
        git_editor::output::set_json_mode(true);
        colored::control::set_override(false);
        if args.range && args.select.is_none() {
            return Err(
                "--json with -x needs --select (the interactive table cannot run in JSON mode)"
                    .into(),
            );
        }
        if args.pick_specific_commits && args.commit.is_none() {
            return Err(
                "--json with -p needs --commit (the interactive menu cannot run in JSON mode)"
                    .into(),
            );
        }
    }

    args.ensure_all_args_present()?;
    validate_inputs(&args)?;

    match determine_operation_mode(&args) {
        OperationMode::Docs => execute_docs_operation(args.docs_out.as_deref()),
        OperationMode::Range => execute_range_operation(&args),
        OperationMode::PickSpecific => execute_pick_specific_operation(&args),
        OperationMode::ShowHistory => execute_show_history_operation(&args),
        OperationMode::FullRewrite => execute_full_rewrite_operation(&mut args),
        OperationMode::Simulate => execute_simulation_operation(&mut args),
    }?;

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

fn execute_docs_operation(out: Option<&str>) -> Result<()> {
    git_editor::docs::execute_docs_operation(out)
}

fn execute_range_operation(args: &Args) -> Result<()> {
    git_editor::say!("{}", "Editing commit range...".cyan());
    rewrite_range_commits(args)
}

fn execute_pick_specific_operation(args: &Args) -> Result<()> {
    git_editor::say!("{}", "Picking specific commits...".cyan());
    rewrite_specific_commits(args)
}

fn execute_show_history_operation(args: &Args) -> Result<()> {
    use git_editor::utils::commit_history::get_commit_history;
    if git_editor::output::json_mode() {
        let commits = get_commit_history(args, false)?;
        let repo = git2::Repository::open(args.repo_path.as_ref().unwrap())?;
        let head = repo.head().ok();
        let branch = head
            .as_ref()
            .filter(|h| h.is_branch())
            .and_then(|h| h.shorthand().ok().map(str::to_string));
        let head_oid = head.and_then(|h| h.target()).map(|o| o.to_string());
        git_editor::output::emit(&git_editor::output::history_json(
            branch.as_deref(),
            head_oid,
            &commits,
        ));
        return Ok(());
    }
    git_editor::say!("{}", "Showing commit history...".cyan());
    get_commit_history(args, true)?;
    Ok(())
}

fn execute_full_rewrite_operation(args: &mut Args) -> Result<()> {
    use git_editor::rewrite::rewrite_all::{apply_plan, begin_offset, full_rewrite_plan};
    use git_editor::utils::commit_history::get_commit_history;
    use git_editor::utils::prompt::{cancelled, confirm};
    use git_editor::utils::simulation::{report_simulation, simulation_from_plan};

    // First, show a summary of what will be changed
    git_editor::say!("{}", "📊 SUMMARY OF PLANNED CHANGES".bold().cyan());
    git_editor::say!("{}", "Analyzing repository...".cyan());

    let commits = get_commit_history(args, false)?;
    if commits.is_empty() {
        git_editor::say!("{}", "No commits found in repository.".yellow());
        return Ok(());
    }
    // The branch tip the preview is based on; the rewrite refuses to run if it moves.
    let repo = git2::Repository::open(args.repo_path.as_ref().unwrap())?;
    let head = git_editor::rewrite::engine::current_branch(&repo)?.head;

    // Check if user wants to keep original timestamps
    if args.should_keep_original_timestamps() {
        git_editor::say!("{}", "✅ Keeping original timestamps as requested.".green());

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
        report_simulation(&simulation_result, args.show_diff, false);

        // Ask for confirmation
        git_editor::say!(
            "\n{}",
            "⚠️  This operation will rewrite Git history!"
                .yellow()
                .bold()
        );
        git_editor::say!(
            "{}",
            "Only author information will be changed, timestamps will remain the same.".cyan()
        );
        git_editor::say!(
            "{}",
            "Make sure you have backed up your repository.".yellow()
        );

        if !confirm("\nDo you want to proceed?", args.yes)? {
            return Err(cancelled());
        }

        git_editor::say!(
            "{}",
            "\n🚀 Proceeding with author information update..."
                .green()
                .bold()
        );
        git_editor::say!("{}", "Updating author information...".cyan());

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
        report_simulation(&simulation_result, args.show_diff, false);

        // Ask for confirmation
        git_editor::say!(
            "\n{}",
            "⚠️  This operation will rewrite Git history permanently!"
                .yellow()
                .bold()
        );
        git_editor::say!(
            "{}",
            "Make sure you have backed up your repository.".yellow()
        );

        if !confirm("\nDo you want to proceed?", args.yes)? {
            return Err(cancelled());
        }

        git_editor::say!("{}", "\n🚀 Proceeding with rewrite...".green().bold());
        git_editor::say!("{}", "Rewriting commits...".cyan());
        apply_plan(&repo, &plan, head)?;
        Ok(())
    }
}

fn execute_simulation_operation(args: &mut Args) -> Result<()> {
    use git_editor::rewrite::engine::current_branch;
    use git_editor::rewrite::rewrite_all::{begin_offset, full_rewrite_plan};
    use git_editor::utils::commit_history::get_commit_history;
    use git_editor::utils::simulation::{report_simulation, simulation_from_plan};

    git_editor::say!("{}", "🔍 SIMULATION MODE".bold().cyan());
    git_editor::say!("{}", "Analyzing repository to preview changes...".cyan());

    let commits = get_commit_history(args, false)?;

    if commits.is_empty() {
        git_editor::say!("{}", "No commits found in repository.".yellow());
        return Ok(());
    }

    // Pick and range modes run their normal selection/editing and stop before writing.
    if args.range {
        return rewrite_range_commits(args);
    }
    if args.pick_specific_commits {
        return rewrite_specific_commits(args);
    }

    let simulation_result = {
        // Full rewrite simulation - check if we have the required arguments
        let dates_given = args.keep_dates || (args.start.is_some() && args.end.is_some());
        if let (Some(name), Some(email), true) =
            (args.name.clone(), args.email.clone(), dates_given)
        {
            // We have all required arguments, do full simulation
            let timestamps = if args.keep_dates {
                None
            } else {
                Some(generate_timestamps(args)?)
            };
            let repo = git2::Repository::open(args.repo_path.as_ref().unwrap())?;
            let head = current_branch(&repo)?.head;
            let plan = full_rewrite_plan(
                &repo,
                head,
                &name,
                &email,
                timestamps.as_deref(),
                begin_offset(args),
                args.committer.into(),
            )?;
            simulation_from_plan(&commits, &plan, "Full Repository Rewrite")
        } else {
            // Missing required arguments - show what's needed
            git_editor::say!(
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

            if git_editor::output::json_mode() {
                return Err(format!(
                    "Missing required arguments for simulation: {}",
                    missing.join(", ")
                )
                .into());
            }
            if !missing.is_empty() {
                git_editor::say!(
                    "{} {}",
                    "Missing required arguments:".red(),
                    missing.join(", ").yellow()
                );
                git_editor::say!("{}", "\nExample usage:".bold());
                git_editor::say!(
                    "{}",
                    "git-editor --simulate --name \"Your Name\" --email \"your@email.com\" \\"
                        .cyan()
                );
                git_editor::say!(
                    "{}",
                    "    --begin \"2023-01-01 09:00:00\" --end \"2023-12-31 17:00:00\"".cyan()
                );
                git_editor::say!();
            }

            // Still show basic repository info
            use git_editor::utils::simulation::{SimulationResult, SimulationStats};
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

    report_simulation(&simulation_result, args.show_diff, true);
    Ok(())
}
