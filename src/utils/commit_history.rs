use crate::utils::types::Result;
use crate::{args::Args, utils::types::CommitInfo};
use colored::Colorize;
use git2::{Repository, Sort};

pub fn get_commit_history(args: &Args, print: bool) -> Result<Vec<CommitInfo>> {
    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

    // Collect all commits first
    let mut commits = Vec::new();
    let mut commit_infos = Vec::new();

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let timestamp = commit.time();
        let datetime = chrono::DateTime::from_timestamp(timestamp.seconds(), 0)
            .unwrap_or_default()
            .naive_utc();

        let commit_info = CommitInfo {
            oid,
            short_hash: oid.to_string()[..8].to_string(),
            timestamp: datetime,
            author_name: commit.author().name().unwrap_or("Unknown").to_string(),
            author_email: commit
                .author()
                .email()
                .unwrap_or("unknown@email.com")
                .to_string(),
            message: commit.message().unwrap_or("(no message)").to_string(),
            parent_count: commit.parent_count(),
        };

        if print {
            commits.push((oid, commit));
        }
        commit_infos.push(commit_info);
    }

    // If print is true, calculate and display statistics
    if print {
        let total_commits = commit_infos.len();

        if total_commits > 0 {
            let timestamps: Vec<_> = commit_infos.iter().map(|c| c.timestamp).collect();
            let mut sorted_timestamps = timestamps.clone();
            sorted_timestamps.sort();

            let earliest_date = sorted_timestamps[0];
            let latest_date = sorted_timestamps[sorted_timestamps.len() - 1];
            let date_span = latest_date.signed_duration_since(earliest_date).num_days();

            let unique_authors: std::collections::HashSet<String> =
                commit_infos.iter().map(|c| c.author_name.clone()).collect();

            // Print summary
            println!("\n{}", "Updated Commit History Summary:".bold().green());
            println!("{}", "-".repeat(60).cyan());
            println!(
                "{}: {}",
                "Total Commits".bold(),
                total_commits.to_string().yellow()
            );
            println!(
                "{}: {} days",
                "Date Span".bold(),
                date_span.to_string().yellow()
            );
            println!(
                "{}: {} to {}",
                "Date Range".bold(),
                earliest_date.format("%Y-%m-%d %H:%M:%S").to_string().blue(),
                latest_date.format("%Y-%m-%d %H:%M:%S").to_string().blue()
            );
            println!(
                "{}: {}",
                "Unique Authors".bold(),
                unique_authors.len().to_string().yellow()
            );
            if unique_authors.len() <= 5 {
                println!(
                    "{}: {}",
                    "Authors".bold(),
                    unique_authors
                        .iter()
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ")
                        .magenta()
                );
            }
            println!("{}", "=".repeat(60).cyan());

            // Print detailed commit history
            println!("\n{}", "Detailed Commit History:".bold().green());
            println!("{}", "-".repeat(60).cyan());

            for commit_info in &commit_infos {
                println!(
                    "{} {} {} {}",
                    commit_info.short_hash.yellow().bold(),
                    commit_info
                        .timestamp
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string()
                        .blue(),
                    commit_info.author_name.magenta(),
                    commit_info.message.lines().next().unwrap_or("").white()
                );
            }

            println!("{}", "=".repeat(60).cyan());
        }
    }

    Ok(commit_infos)
}
