use crate::utils::types::Result;
use crate::{args::Args, utils::types::CommitInfo};
use colored::Colorize;
use git2::{Repository, Sort};

pub fn print_updated_history(args: &Args) -> Result<()> {
    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

    // Collect all commits first to calculate statistics
    let mut commits = Vec::new();
    let mut timestamps = Vec::new();

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        let timestamp = commit.time();
        let datetime = chrono::DateTime::from_timestamp(timestamp.seconds(), 0)
            .unwrap_or_default()
            .naive_utc();

        commits.push((oid, commit));
        timestamps.push(datetime);
    }

    // Calculate statistics
    let total_commits = commits.len();
    let (earliest_date, latest_date) = if !timestamps.is_empty() {
        let mut sorted_timestamps = timestamps.clone();
        sorted_timestamps.sort();
        (
            sorted_timestamps[0],
            sorted_timestamps[sorted_timestamps.len() - 1],
        )
    } else {
        return Ok(());
    };

    let date_span = latest_date.signed_duration_since(earliest_date).num_days();
    let unique_authors: std::collections::HashSet<String> = commits
        .iter()
        .map(|(_, commit)| commit.author().name().unwrap_or("Unknown").to_string())
        .collect();

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

    for (i, (oid, commit)) in commits.iter().enumerate() {
        let short_hash = &oid.to_string()[..8];
        let message = commit.message().unwrap_or("(no message)");
        let author = commit.author();

        println!(
            "{} {} {} {}",
            short_hash.yellow().bold(),
            timestamps[i].format("%Y-%m-%d %H:%M:%S").to_string().blue(),
            author.name().unwrap_or("Unknown").magenta(),
            message.lines().next().unwrap_or("").white()
        );
    }

    println!("{}", "=".repeat(60).cyan());

    Ok(())
}

pub fn get_commit_history(args: &Args) -> Result<Vec<CommitInfo>> {
    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

    let mut commits = Vec::new();

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

        commits.push(commit_info);
    }

    Ok(commits)
}
