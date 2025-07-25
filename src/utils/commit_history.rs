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
            let file_path = temp_dir.path().join(format!("test{}.txt", i));
            fs::write(&file_path, format!("test content {}", i)).unwrap();

            let mut index = repo.index().unwrap();
            index
                .add_path(std::path::Path::new(&format!("test{}.txt", i)))
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
                &format!("Commit {}", i),
                &tree,
                &parents.iter().collect::<Vec<_>>(),
            )
            .unwrap();
        }

        (temp_dir, repo_path)
    }

    #[test]
    fn test_get_commit_history_without_print() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pic_specific_commits: false,
        };

        let result = get_commit_history(&args, false);
        assert!(result.is_ok());

        let commit_infos = result.unwrap();
        assert_eq!(commit_infos.len(), 3);

        // Check that commits are in reverse chronological order (newest first)
        assert_eq!(commit_infos[0].message, "Commit 3");
        assert_eq!(commit_infos[1].message, "Commit 2");
        assert_eq!(commit_infos[2].message, "Commit 1");
    }

    #[test]
    fn test_get_commit_history_with_print() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: true,
            pic_specific_commits: false,
        };

        let result = get_commit_history(&args, true);
        assert!(result.is_ok());

        let commit_infos = result.unwrap();
        assert_eq!(commit_infos.len(), 3);
    }

    #[test]
    fn test_commit_info_fields() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pic_specific_commits: false,
        };

        let result = get_commit_history(&args, false);
        assert!(result.is_ok());

        let commit_infos = result.unwrap();
        let first_commit = &commit_infos[0];

        // Check that all fields are populated correctly
        assert!(!first_commit.short_hash.is_empty());
        assert_eq!(first_commit.short_hash.len(), 8);
        assert_eq!(first_commit.author_name, "Test User");
        assert_eq!(first_commit.author_email, "test@example.com");
        assert!(!first_commit.message.is_empty());
        assert_eq!(first_commit.parent_count, 1); // Second and third commits have parents
    }

    #[test]
    fn test_get_commit_history_empty_repo() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();

        // Initialize empty git repo
        git2::Repository::init(&repo_path).unwrap();

        let args = Args {
            repo_path: Some(repo_path),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pic_specific_commits: false,
        };

        let result = get_commit_history(&args, false);
        // Empty repo should return error because there's no HEAD
        assert!(result.is_err());
    }

    #[test]
    fn test_get_commit_history_invalid_repo() {
        let args = Args {
            repo_path: Some("/nonexistent/path".to_string()),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pic_specific_commits: false,
        };

        let result = get_commit_history(&args, false);
        assert!(result.is_err());
    }

    #[test]
    fn test_commit_info_parent_count() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path),
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pic_specific_commits: false,
        };

        let result = get_commit_history(&args, false);
        assert!(result.is_ok());

        let commit_infos = result.unwrap();

        // First commit (chronologically) should have 0 parents
        assert_eq!(commit_infos[2].parent_count, 0);

        // Second and third commits should have 1 parent each
        assert_eq!(commit_infos[1].parent_count, 1);
        assert_eq!(commit_infos[0].parent_count, 1);
    }
}
