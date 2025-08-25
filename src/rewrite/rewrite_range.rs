use crate::utils::types::CommitInfo;
use crate::utils::types::Result;
use crate::{args::Args, utils::commit_history::get_commit_history};
use chrono::NaiveDateTime;
use colored::Colorize;
use git2::{Repository, Signature, Sort, Time};
use std::collections::HashMap;
use std::io::{self, Write};

pub fn parse_range_input(input: &str) -> Result<(usize, usize)> {
    let parts: Vec<&str> = input.trim().split('-').collect();

    if parts.len() != 2 {
        return Err("Invalid range format. Use format like '5-11'".into());
    }

    let start = parts[0]
        .trim()
        .parse::<usize>()
        .map_err(|_| "Invalid start number in range")?;
    let end = parts[1]
        .trim()
        .parse::<usize>()
        .map_err(|_| "Invalid end number in range")?;

    if start < 1 {
        return Err("Start position must be 1 or greater".into());
    }

    if end < start {
        return Err("End position must be greater than or equal to start position".into());
    }

    Ok((start, end))
}

pub fn select_commit_range(commits: &[CommitInfo]) -> Result<(usize, usize)> {
    println!("\n{}", "Commit History:".bold().green());
    println!("{}", "-".repeat(80).cyan());

    for (i, commit) in commits.iter().enumerate() {
        println!(
            "{:3}. {} {} {} {}",
            i + 1,
            commit.short_hash.yellow().bold(),
            commit
                .timestamp
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .blue(),
            commit.author_name.magenta(),
            commit.message.lines().next().unwrap_or("").white()
        );
    }

    println!("{}", "-".repeat(80).cyan());
    println!(
        "\n{}",
        "Enter range in format 'start-end' (e.g., '5-11'):"
            .bold()
            .green()
    );
    print!("{} ", "Range:".bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let (start, end) = parse_range_input(&input)?;

    if start > commits.len() || end > commits.len() {
        return Err(format!(
            "Range out of bounds. Available commits: 1-{}",
            commits.len()
        )
        .into());
    }

    Ok((start - 1, end - 1)) // Convert to 0-based indexing
}

pub fn show_range_details(commits: &[CommitInfo], start_idx: usize, end_idx: usize) -> Result<()> {
    println!("\n{}", "Selected Commit Range:".bold().green());
    println!("{}", "=".repeat(80).cyan());

    for (idx, commit) in commits[start_idx..=end_idx].iter().enumerate() {
        println!(
            "\n{}: {} ({})",
            format!("Commit {}", start_idx + idx + 1).bold(),
            commit.short_hash.yellow(),
            &commit.oid.to_string()[..8]
        );
        println!(
            "{}: {}",
            "Author".bold(),
            format!("{} <{}>", commit.author_name, commit.author_email).magenta()
        );
        println!(
            "{}: {}",
            "Date".bold(),
            commit
                .timestamp
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .blue()
        );
        println!(
            "{}: {}",
            "Message".bold(),
            commit.message.lines().next().unwrap_or("").white()
        );
    }

    println!("\n{}", "=".repeat(80).cyan());
    println!(
        "{} {} commits selected for editing",
        "Total:".bold(),
        (end_idx - start_idx + 1).to_string().green()
    );

    Ok(())
}

pub fn get_range_edit_info(args: &Args) -> Result<(String, String, NaiveDateTime, NaiveDateTime)> {
    println!("\n{}", "Range Edit Configuration:".bold().green());

    // Get author name
    let author_name = if let Some(name) = &args.name {
        name.clone()
    } else {
        print!("{} ", "New author name:".bold());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    // Get author email
    let author_email = if let Some(email) = &args.email {
        email.clone()
    } else {
        print!("{} ", "New author email:".bold());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        input.trim().to_string()
    };

    // Get start timestamp
    let start_timestamp = if let Some(start) = &args.start {
        NaiveDateTime::parse_from_str(start, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid start timestamp format")?
    } else {
        print!("{} ", "Start timestamp (YYYY-MM-DD HH:MM:SS):".bold());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        NaiveDateTime::parse_from_str(input.trim(), "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid start timestamp format")?
    };

    // Get end timestamp
    let end_timestamp = if let Some(end) = &args.end {
        NaiveDateTime::parse_from_str(end, "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid end timestamp format")?
    } else {
        print!("{} ", "End timestamp (YYYY-MM-DD HH:MM:SS):".bold());
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        NaiveDateTime::parse_from_str(input.trim(), "%Y-%m-%d %H:%M:%S")
            .map_err(|_| "Invalid end timestamp format")?
    };

    if end_timestamp <= start_timestamp {
        return Err("End timestamp must be after start timestamp".into());
    }

    Ok((author_name, author_email, start_timestamp, end_timestamp))
}

pub fn generate_range_timestamps(
    start_time: NaiveDateTime,
    end_time: NaiveDateTime,
    count: usize,
) -> Vec<NaiveDateTime> {
    if count == 0 {
        return vec![];
    }

    if count == 1 {
        return vec![start_time];
    }

    let total_duration = end_time.signed_duration_since(start_time);
    let step_duration = total_duration / (count - 1) as i32;

    (0..count)
        .map(|i| start_time + step_duration * i as i32)
        .collect()
}

pub fn rewrite_range_commits(args: &Args) -> Result<()> {
    let commits = get_commit_history(args, false)?;

    if commits.is_empty() {
        println!("{}", "No commits found!".red());
        return Ok(());
    }

    let (start_idx, end_idx) = select_commit_range(&commits)?;
    show_range_details(&commits, start_idx, end_idx)?;

    let (author_name, author_email, start_time, end_time) = get_range_edit_info(args)?;

    let range_size = end_idx - start_idx + 1;
    let timestamps = generate_range_timestamps(start_time, end_time, range_size);

    // Show planned changes
    println!("\n{}", "Planned Changes:".bold().yellow());
    for (i, (commit, timestamp)) in commits[start_idx..=end_idx]
        .iter()
        .zip(timestamps.iter())
        .enumerate()
    {
        println!(
            "  {}: {} -> {}",
            format!("Commit {}", start_idx + i + 1).bold(),
            format!(
                "{} <{}> {}",
                commit.author_name,
                commit.author_email,
                commit.timestamp.format("%Y-%m-%d %H:%M:%S")
            )
            .red(),
            format!(
                "{} <{}> {}",
                author_name,
                author_email,
                timestamp.format("%Y-%m-%d %H:%M:%S")
            )
            .green()
        );
    }

    print!("\n{} (y/n): ", "Proceed with changes?".bold());
    io::stdout().flush()?;

    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;

    if confirm.trim().to_lowercase() != "y" {
        println!("{}", "Operation cancelled.".yellow());
        return Ok(());
    }

    // Apply changes
    apply_range_changes(
        args,
        &commits,
        start_idx,
        end_idx,
        &author_name,
        &author_email,
        &timestamps,
    )?;

    println!("\n{}", "✓ Commit range successfully edited!".green().bold());

    if args.show_history {
        get_commit_history(args, true)?;
    }

    Ok(())
}

fn apply_range_changes(
    args: &Args,
    _commits: &[CommitInfo],
    start_idx: usize,
    end_idx: usize,
    author_name: &str,
    author_email: &str,
    timestamps: &[NaiveDateTime],
) -> Result<()> {
    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;
    let head_ref = repo.head()?;
    let branch_name = head_ref
        .shorthand()
        .ok_or("Detached HEAD or invalid branch")?;
    let full_ref = format!("refs/heads/{branch_name}");

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    let mut orig_oids: Vec<_> = revwalk.filter_map(|id| id.ok()).collect();
    orig_oids.reverse();

    let mut new_map: HashMap<git2::Oid, git2::Oid> = HashMap::new();
    let mut last_new_oid = None;
    let mut range_timestamp_idx = 0;

    for (commit_idx, &oid) in orig_oids.iter().enumerate() {
        let orig = repo.find_commit(oid)?;
        let tree = orig.tree()?;

        let new_parents: Result<Vec<_>> = orig
            .parent_ids()
            .map(|pid| {
                let new_pid = *new_map.get(&pid).unwrap_or(&pid);
                repo.find_commit(new_pid).map_err(|e| e.into())
            })
            .collect();

        let new_oid = if commit_idx >= start_idx && commit_idx <= end_idx {
            // This commit is in our range - update it
            let timestamp = timestamps[range_timestamp_idx];
            range_timestamp_idx += 1;

            let sig = Signature::new(
                author_name,
                author_email,
                &Time::new(timestamp.and_utc().timestamp(), 0),
            )?;

            repo.commit(
                None,
                &sig,
                &sig,
                orig.message().unwrap_or_default(),
                &tree,
                &new_parents?.iter().collect::<Vec<_>>(),
            )?
        } else {
            // Keep other commits as-is but update parent references
            let author = orig.author();
            let committer = orig.committer();

            repo.commit(
                None,
                &author,
                &committer,
                orig.message().unwrap_or_default(),
                &tree,
                &new_parents?.iter().collect::<Vec<_>>(),
            )?
        };

        new_map.insert(oid, new_oid);
        last_new_oid = Some(new_oid);
    }

    if let Some(new_head) = last_new_oid {
        repo.reference(&full_ref, new_head, true, "edited commit range")?;
        println!(
            "{} '{}' -> {}",
            "Updated branch".green(),
            branch_name.cyan(),
            new_head.to_string()[..8].to_string().cyan()
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo_with_commits() -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();

        // Initialize git repo
        let repo = git2::Repository::init(&repo_path).unwrap();

        // Create multiple commits
        for i in 1..=5 {
            let file_path = temp_dir.path().join(format!("test{i}.txt"));
            fs::write(&file_path, format!("test content {i}")).unwrap();

            let mut index = repo.index().unwrap();
            index
                .add_path(std::path::Path::new(&format!("test{i}.txt")))
                .unwrap();
            index.write().unwrap();

            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();

            let sig = git2::Signature::new(
                "Test User",
                "test@example.com",
                &git2::Time::new(1234567890 + i as i64 * 3600, 0),
            )
            .unwrap();

            let parents = if i == 1 {
                vec![]
            } else {
                let head = repo.head().unwrap();
                let parent_commit = head.peel_to_commit().unwrap();
                vec![parent_commit]
            };

            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                &format!("Commit {i}"),
                &tree,
                &parents.iter().collect::<Vec<_>>(),
            )
            .unwrap();
        }

        (temp_dir, repo_path)
    }

    #[test]
    fn test_parse_range_input_valid() {
        let result = parse_range_input("5-11");
        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 5);
        assert_eq!(end, 11);
    }

    #[test]
    fn test_parse_range_input_with_spaces() {
        let result = parse_range_input(" 3 - 8 ");
        assert!(result.is_ok());
        let (start, end) = result.unwrap();
        assert_eq!(start, 3);
        assert_eq!(end, 8);
    }

    #[test]
    fn test_parse_range_input_invalid_format() {
        let result = parse_range_input("5");
        assert!(result.is_err());

        let result = parse_range_input("5-11-15");
        assert!(result.is_err());

        let result = parse_range_input("abc-def");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_range_input_invalid_range() {
        let result = parse_range_input("11-5");
        assert!(result.is_err());

        let result = parse_range_input("0-5");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_range_timestamps() {
        let start =
            NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end =
            NaiveDateTime::parse_from_str("2023-01-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        let timestamps = generate_range_timestamps(start, end, 5);

        assert_eq!(timestamps.len(), 5);
        assert_eq!(timestamps[0], start);
        assert_eq!(timestamps[4], end);

        // Check that timestamps are evenly distributed
        for i in 1..timestamps.len() {
            assert!(timestamps[i] >= timestamps[i - 1]);
        }
    }

    #[test]
    fn test_generate_range_timestamps_edge_cases() {
        let start =
            NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end =
            NaiveDateTime::parse_from_str("2023-01-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        // Zero count
        let timestamps = generate_range_timestamps(start, end, 0);
        assert_eq!(timestamps.len(), 0);

        // Single timestamp
        let timestamps = generate_range_timestamps(start, end, 1);
        assert_eq!(timestamps.len(), 1);
        assert_eq!(timestamps[0], start);
    }

    #[test]
    fn test_rewrite_range_commits_with_repo() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path),
            email: Some("new@example.com".to_string()),
            name: Some("New User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-01 10:00:00".to_string()),
            show_history: false,
            pic_specific_commits: false,
            range: false,
        };

        // Test that get_commit_history returns commits for this repo
        let commits = get_commit_history(&args, false).unwrap();
        assert_eq!(commits.len(), 5);

        // Test range validation
        let (start, end) = (0, 2); // 0-based indexing
        assert!(start <= end);
        assert!(end < commits.len());

        // Test timestamp generation
        let start_time =
            NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_time =
            NaiveDateTime::parse_from_str("2023-01-01 10:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let timestamps = generate_range_timestamps(start_time, end_time, 3);
        assert_eq!(timestamps.len(), 3);
    }
}
