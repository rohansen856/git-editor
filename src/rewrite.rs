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

fn print_updated_history(args: &Args) -> Result<()> {
    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;
    
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    
    println!("\n{}", "Updated Commit History:".bold().green());
    println!("{}", "=".repeat(50).cyan());
    
    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;
        
        let timestamp = commit.time();
        let datetime = chrono::DateTime::from_timestamp(timestamp.seconds(), 0)
            .unwrap_or_default()
            .naive_utc();
        
        let short_hash = &oid.to_string()[..8];
        let message = commit.message().unwrap_or("(no message)");
        let author = commit.author();
        
        println!(
            "{} {} {} {}",
            short_hash.yellow().bold(),
            datetime.format("%Y-%m-%d %H:%M:%S").to_string().blue(),
            author.name().unwrap_or("Unknown").magenta(),
            message.lines().next().unwrap_or("").white()
        );
    }
    
    println!("{}", "=".repeat(50).cyan());
    
    Ok(())
}