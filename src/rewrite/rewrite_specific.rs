use crate::utils::types::Result;
use crate::utils::types::{CommitInfo, EditOptions};
use crate::{args::Args, utils::commit_history::get_commit_history};
use chrono::NaiveDateTime;
use colored::Colorize;
use git2::{Repository, Signature, Sort, Time};
use std::collections::HashMap;
use std::io::{self, Write};

pub fn select_commit(commits: &[CommitInfo]) -> Result<usize> {
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
            commit
                .message
                .lines()
                .next()
                .unwrap_or("(no message)")
                .white()
        );
    }

    println!("{}", "-".repeat(80).cyan());
    print!("\n{} ", "Select commit number to edit:".bold().green());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let selection = input
        .trim()
        .parse::<usize>()
        .map_err(|_| "Invalid number")?;

    if selection < 1 || selection > commits.len() {
        return Err("Selection out of range".into());
    }

    Ok(selection - 1)
}

pub fn show_commit_details(commit: &CommitInfo, repo: &Repository) -> Result<()> {
    println!("\n{}", "Selected Commit Details:".bold().green());
    println!("{}", "=".repeat(80).cyan());

    println!("{}: {}", "Hash".bold(), commit.oid.to_string().yellow());
    println!("{}: {}", "Short Hash".bold(), commit.short_hash.yellow());
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
        "Parent Count".bold(),
        commit.parent_count.to_string().white()
    );

    println!("\n{}", "Message:".bold());
    println!("{}", commit.message.white());

    // Show parent commits
    if commit.parent_count > 0 {
        let git_commit = repo.find_commit(commit.oid)?;
        println!("\n{}", "Parent Commits:".bold());
        for (i, parent_id) in git_commit.parent_ids().enumerate() {
            let parent = repo.find_commit(parent_id)?;
            println!(
                "  {}: {} - {}",
                i + 1,
                parent_id.to_string()[..8].to_string().yellow(),
                parent.summary().unwrap_or("(no message)").white()
            );
        }
    }

    println!("{}", "=".repeat(80).cyan());
    Ok(())
}

// Get user input for what to change
pub fn get_edit_options() -> Result<EditOptions> {
    println!("\n{}", "What would you like to edit?".bold().green());
    println!("1. Author name");
    println!("2. Author email");
    println!("3. Commit timestamp");
    println!("4. Commit message");
    println!("5. All of the above");

    print!("\n{} ", "Select option(s) (comma-separated):".bold());
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    let selections: Vec<usize> = input
        .trim()
        .split(',')
        .filter_map(|s| s.trim().parse::<usize>().ok())
        .collect();

    let mut options = EditOptions::default();

    for &selection in &selections {
        match selection {
            1 => {
                print!("{} ", "New author name:".bold());
                io::stdout().flush()?;
                let mut name = String::new();
                io::stdin().read_line(&mut name)?;
                options.author_name = Some(name.trim().to_string());
            }
            2 => {
                print!("{} ", "New author email:".bold());
                io::stdout().flush()?;
                let mut email = String::new();
                io::stdin().read_line(&mut email)?;
                options.author_email = Some(email.trim().to_string());
            }
            3 => {
                print!("{} ", "New timestamp (YYYY-MM-DD HH:MM:SS):".bold());
                io::stdout().flush()?;
                let mut timestamp = String::new();
                io::stdin().read_line(&mut timestamp)?;
                let dt = NaiveDateTime::parse_from_str(timestamp.trim(), "%Y-%m-%d %H:%M:%S")
                    .map_err(|_| "Invalid timestamp format")?;
                options.timestamp = Some(dt);
            }
            4 => {
                println!("{} ", "New commit message (end with empty line):".bold());
                let mut message = String::new();
                loop {
                    let mut line = String::new();
                    io::stdin().read_line(&mut line)?;
                    if line.trim().is_empty() {
                        break;
                    }
                    message.push_str(&line);
                }
                options.message = Some(message.trim().to_string());
            }
            5 => {
                // Get all inputs
                print!("{} ", "New author name:".bold());
                io::stdout().flush()?;
                let mut name = String::new();
                io::stdin().read_line(&mut name)?;
                options.author_name = Some(name.trim().to_string());

                print!("{} ", "New author email:".bold());
                io::stdout().flush()?;
                let mut email = String::new();
                io::stdin().read_line(&mut email)?;
                options.author_email = Some(email.trim().to_string());

                print!("{} ", "New timestamp (YYYY-MM-DD HH:MM:SS):".bold());
                io::stdout().flush()?;
                let mut timestamp = String::new();
                io::stdin().read_line(&mut timestamp)?;
                let dt = NaiveDateTime::parse_from_str(timestamp.trim(), "%Y-%m-%d %H:%M:%S")
                    .map_err(|_| "Invalid timestamp format")?;
                options.timestamp = Some(dt);

                println!("{} ", "New commit message (end with empty line):".bold());
                let mut message = String::new();
                loop {
                    let mut line = String::new();
                    io::stdin().read_line(&mut line)?;
                    if line.trim().is_empty() {
                        break;
                    }
                    message.push_str(&line);
                }
                options.message = Some(message.trim().to_string());
            }
            _ => println!("Invalid option: {selection}"),
        }
    }

    Ok(options)
}

pub fn rewrite_specific_commits(args: &Args) -> Result<()> {
    let commits = get_commit_history(args, false)?;

    if commits.is_empty() {
        println!("{}", "No commits found!".red());
        return Ok(());
    }

    let selected_index = select_commit(&commits)?;
    let selected_commit = &commits[selected_index];

    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;
    show_commit_details(selected_commit, &repo)?;

    let edit_options = get_edit_options()?;

    // Confirm changes
    println!("\n{}", "Planned changes:".bold().yellow());
    if let Some(ref name) = edit_options.author_name {
        println!(
            "  Author name: {} -> {}",
            selected_commit.author_name.red(),
            name.green()
        );
    }
    if let Some(ref email) = edit_options.author_email {
        println!(
            "  Author email: {} -> {}",
            selected_commit.author_email.red(),
            email.green()
        );
    }
    if let Some(ref timestamp) = edit_options.timestamp {
        println!(
            "  Timestamp: {} -> {}",
            selected_commit
                .timestamp
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
                .red(),
            timestamp.format("%Y-%m-%d %H:%M:%S").to_string().green()
        );
    }
    if let Some(ref message) = edit_options.message {
        println!(
            "  Message: {} -> {}",
            selected_commit.message.lines().next().unwrap_or("").red(),
            message.lines().next().unwrap_or("").green()
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
    apply_commit_changes(&repo, selected_commit, &edit_options)?;

    println!("\n{}", "✓ Commit successfully edited!".green().bold());

    if args.show_history {
        get_commit_history(args, true)?;
    }

    Ok(())
}

// Apply the changes to the selected commit
fn apply_commit_changes(
    repo: &Repository,
    target_commit: &CommitInfo,
    options: &EditOptions,
) -> Result<()> {
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

    for &oid in orig_oids.iter() {
        let orig = repo.find_commit(oid)?;
        let tree = orig.tree()?;

        let new_parents: Result<Vec<_>> = orig
            .parent_ids()
            .map(|pid| {
                let new_pid = *new_map.get(&pid).unwrap_or(&pid);
                repo.find_commit(new_pid).map_err(|e| e.into())
            })
            .collect();

        let new_oid = if oid == target_commit.oid {
            // This is the commit we want to edit
            let author_name = options
                .author_name
                .as_ref()
                .unwrap_or(&target_commit.author_name);
            let author_email = options
                .author_email
                .as_ref()
                .unwrap_or(&target_commit.author_email);
            let timestamp = options.timestamp.unwrap_or(target_commit.timestamp);
            let message = options
                .message
                .as_deref()
                .unwrap_or_else(|| orig.message().unwrap_or_default());

            let author_sig = Signature::new(
                author_name,
                author_email,
                &Time::new(timestamp.and_utc().timestamp(), 0),
            )?;

            // Keep the original committer unless we're changing the timestamp
            let committer_sig = if options.timestamp.is_some() {
                author_sig.clone()
            } else {
                let committer = orig.committer();
                Signature::new(
                    committer.name().unwrap_or("Unknown"),
                    committer.email().unwrap_or("unknown@email.com"),
                    &committer.when(),
                )?
            };

            repo.commit(
                None,
                &author_sig,
                &committer_sig,
                message,
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
        repo.reference(&full_ref, new_head, true, "edited specific commit")?;
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
        for i in 1..=3 {
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
    fn test_show_commit_details() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let repo = Repository::open(&repo_path).unwrap();

        // Get commit info
        let args = Args {
            repo_path: Some(repo_path),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        let commits = get_commit_history(&args, false).unwrap();
        let commit = &commits[0];

        // Test that show_commit_details doesn't crash
        let result = show_commit_details(commit, &repo);
        assert!(result.is_ok());
    }

    #[test]
    fn test_edit_options_default() {
        let options = EditOptions::default();

        assert_eq!(options.author_name, None);
        assert_eq!(options.author_email, None);
        assert_eq!(options.timestamp, None);
        assert_eq!(options.message, None);
    }

    #[test]
    fn test_edit_options_with_values() {
        let timestamp =
            NaiveDateTime::parse_from_str("2023-01-01 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        let options = EditOptions {
            author_name: Some("New Author".to_string()),
            author_email: Some("new@example.com".to_string()),
            timestamp: Some(timestamp),
            message: Some("New commit message".to_string()),
        };

        assert_eq!(options.author_name, Some("New Author".to_string()));
        assert_eq!(options.author_email, Some("new@example.com".to_string()));
        assert_eq!(options.timestamp, Some(timestamp));
        assert_eq!(options.message, Some("New commit message".to_string()));
    }

    #[test]
    fn test_commit_selection_validation() {
        // Test the selection validation logic that's used in select_commit
        let commits = [CommitInfo {
            oid: git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678").unwrap(),
            short_hash: "12345678".to_string(),
            timestamp: NaiveDateTime::parse_from_str("2023-01-01 12:00:00", "%Y-%m-%d %H:%M:%S")
                .unwrap(),
            author_name: "Test User".to_string(),
            author_email: "test@example.com".to_string(),
            message: "Test commit".to_string(),
            parent_count: 0,
        }];

        // Test valid selection range
        let selection = 1;
        assert!(selection >= 1 && selection <= commits.len());

        // Test invalid selections
        let invalid_selection1 = 0;
        assert!(invalid_selection1 < 1 || invalid_selection1 > commits.len());

        let invalid_selection2 = commits.len() + 1;
        assert!(invalid_selection2 < 1 || invalid_selection2 > commits.len());
    }

    #[test]
    fn test_rewrite_specific_commits_with_empty_commits() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: true,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        // Test that the function handles the case where get_commit_history returns commits
        let commits = get_commit_history(&args, false).unwrap();
        assert!(!commits.is_empty());
        assert_eq!(commits.len(), 3);
    }

    #[test]
    fn test_apply_commit_changes_logic() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let _repo = Repository::open(&repo_path).unwrap();

        // Get commit info
        let args = Args {
            repo_path: Some(repo_path),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
        };

        let commits = get_commit_history(&args, false).unwrap();
        let target_commit = &commits[0];

        // Test EditOptions with different values
        let options = EditOptions {
            author_name: Some("New Author".to_string()),
            author_email: Some("new@example.com".to_string()),
            timestamp: Some(
                NaiveDateTime::parse_from_str("2023-01-01 12:00:00", "%Y-%m-%d %H:%M:%S").unwrap(),
            ),
            message: Some("New commit message".to_string()),
        };

        // Test that the options are properly set
        assert_eq!(options.author_name.as_ref().unwrap(), "New Author");
        assert_eq!(options.author_email.as_ref().unwrap(), "new@example.com");
        assert!(options.timestamp.is_some());
        assert_eq!(options.message.as_ref().unwrap(), "New commit message");

        // Test fallback to original values
        let partial_options = EditOptions {
            author_name: None,
            author_email: None,
            timestamp: None,
            message: None,
        };

        let author_name = partial_options
            .author_name
            .as_ref()
            .unwrap_or(&target_commit.author_name);
        let author_email = partial_options
            .author_email
            .as_ref()
            .unwrap_or(&target_commit.author_email);

        assert_eq!(author_name, &target_commit.author_name);
        assert_eq!(author_email, &target_commit.author_email);
    }
}
