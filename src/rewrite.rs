use crate::args::Args;
use crate::types::Result;
use chrono::NaiveDateTime;
use colored::Colorize;
use git2::{Repository, Signature, Sort, Time};
use std::collections::HashMap;

pub fn rewrite_commits(args: &Args, timestamps: Vec<NaiveDateTime>) -> Result<()> {
    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;
    let head_ref = repo.head()?;
    let branch_name = head_ref
        .shorthand()
        .ok_or("Detached HEAD or invalid branch")?;
    let full_ref = format!("refs/heads/{}", branch_name);

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    let mut orig_oids: Vec<_> = revwalk.filter_map(|id| id.ok()).collect();
    orig_oids.reverse();

    let mut new_map: HashMap<git2::Oid, git2::Oid> = HashMap::new();
    let mut last_new_oid = None;

    for (i, &oid) in orig_oids.iter().enumerate() {
        let orig = repo.find_commit(oid)?;
        let tree = orig.tree()?;

        let new_parents: Result<Vec<_>> = orig
            .parent_ids()
            .map(|pid| {
                let new_pid = *new_map.get(&pid).unwrap_or(&pid);
                repo.find_commit(new_pid).map_err(|e| e.into())
            })
            .collect();

        let timestamp: i64 = timestamps[i].and_utc().timestamp();
        let sig = Signature::new(
            args.name.as_ref().unwrap(),
            args.email.as_ref().unwrap(),
            &Time::new(timestamp, 0),
        )?;

        let new_oid = repo.commit(
            None,
            &sig,
            &sig,
            orig.message().unwrap_or_default(),
            &tree,
            &new_parents?.iter().collect::<Vec<_>>(),
        )?;

        new_map.insert(oid, new_oid);
        last_new_oid = Some(new_oid);
    }

    if let Some(new_head) = last_new_oid {
        repo.reference(&full_ref, new_head, true, "history rewritten")?;
        println!(
            "{} '{}' -> {}",
            "Rewritten branch".green(),
            branch_name.cyan(),
            new_head.to_string().cyan()
        );
        if args.show_history {
            print_updated_history(&args)?;
        }
    }

    Ok(())
}

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
        (sorted_timestamps[0], sorted_timestamps[sorted_timestamps.len() - 1])
    } else {
        return Ok(());
    };
    
    let date_span = latest_date.signed_duration_since(earliest_date).num_days();
    let unique_authors: std::collections::HashSet<String> = commits.iter()
        .map(|(_, commit)| commit.author().name().unwrap_or("Unknown").to_string())
        .collect();
    
    // Print summary
    println!("\n{}", "Updated Commit History Summary:".bold().green());
    println!("{}", "-".repeat(60).cyan());
    println!("{}: {}", "Total Commits".bold(), total_commits.to_string().yellow());
    println!("{}: {} days", "Date Span".bold(), date_span.to_string().yellow());
    println!("{}: {} to {}", 
        "Date Range".bold(), 
        earliest_date.format("%Y-%m-%d %H:%M:%S").to_string().blue(),
        latest_date.format("%Y-%m-%d %H:%M:%S").to_string().blue()
    );
    println!("{}: {}", "Unique Authors".bold(), unique_authors.len().to_string().yellow());
    if unique_authors.len() <= 5 {
        println!("{}: {}", "Authors".bold(), 
            unique_authors.iter().cloned().collect::<Vec<_>>().join(", ").magenta());
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