use crate::args::Args;
use crate::utils::types::Result;
use chrono::{Duration, NaiveDateTime};
use colored::*;
use rand::Rng;
use uuid::Uuid;

pub fn generate_timestamps(args: &mut Args) -> Result<Vec<NaiveDateTime>> {
    let start_dt =
        NaiveDateTime::parse_from_str(args.start.as_ref().unwrap(), "%Y-%m-%d %H:%M:%S")?;
    let end_dt = NaiveDateTime::parse_from_str(args.end.as_ref().unwrap(), "%Y-%m-%d %H:%M:%S")?;

    if start_dt >= end_dt {
        eprintln!(
            "{}",
            "Start datetime must be before end datetime".red().bold()
        );
        std::process::exit(1);
    }

    if url::Url::parse(args.repo_path.as_ref().unwrap()).is_ok()
        && !std::path::Path::new(args.repo_path.as_ref().unwrap()).exists()
    {
        let tmp_dir = std::env::temp_dir().join(format!("git_editor_{}", Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_dir)?;

        let status = std::process::Command::new("git")
            .args([
                "clone",
                args.repo_path.as_ref().unwrap(),
                &tmp_dir.to_string_lossy(),
            ])
            .status()?;

        if !status.success() {
            eprintln!("{}", "Failed to clone repository".red().bold());
            std::process::exit(1);
        }

        // Update repo_path to point to the cloned repository
        args.repo_path = Some(tmp_dir.to_string_lossy().to_string());
    }
    let total_commits = count_commits(args.repo_path.as_ref().unwrap())?;
    if total_commits == 0 {
        eprintln!("{}", "No commits found in repository".red().bold());
        std::process::exit(1);
    }

    let min_span = Duration::hours(3 * (total_commits as i64 - 1));
    let total_span = end_dt - start_dt;

    if total_span < min_span {
        eprintln!(
            "{} {} {}",
            "Date range too small for".red().bold(),
            total_commits.to_string().yellow(),
            "commits".red().bold()
        );
        std::process::exit(1);
    }

    let slack = total_span - min_span;
    let mut rng = rand::rng();
    let mut weights: Vec<f64> = (0..(total_commits - 1)).map(|_| rng.random()).collect();
    let sum: f64 = weights.iter().sum();

    for w in &mut weights {
        *w = (*w / sum) * slack.num_seconds() as f64;
    }

    let mut timestamps = Vec::with_capacity(total_commits);
    let mut current = start_dt;
    timestamps.push(current);

    for w in &weights {
        let secs = w.round() as i64 + 3 * 3600;
        current += Duration::seconds(secs);
        timestamps.push(current);
    }

    Ok(timestamps)
}

fn count_commits(repo_path: &str) -> Result<usize> {
    let repo = git2::Repository::open(repo_path)?;
    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;
    Ok(revwalk.count())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_repo() -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();

        // Initialize git repo
        let repo = git2::Repository::init(&repo_path).unwrap();

        // Create a test file
        let file_path = temp_dir.path().join("test.txt");
        fs::write(&file_path, "test content").unwrap();

        // Add and commit file
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();

        let sig = git2::Signature::new(
            "Test User",
            "test@example.com",
            &git2::Time::new(1234567890, 0),
        )
        .unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        (temp_dir, repo_path)
    }

    #[test]
    fn test_count_commits() {
        let (_temp_dir, repo_path) = create_test_repo();
        let count = count_commits(&repo_path).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_generate_timestamps_invalid_date_format() {
        let (_temp_dir, repo_path) = create_test_repo();
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("invalid-date".to_string()),
            end: Some("2023-01-02 00:00:00".to_string()),
            show_history: false,
            pick_specific_commits: false,
            range: false,
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_timestamps_valid_range() {
        let (_temp_dir, repo_path) = create_test_repo();
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-10 00:00:00".to_string()),
            show_history: false,
            pick_specific_commits: false,
            range: false,
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_ok());

        let timestamps = result.unwrap();
        assert_eq!(timestamps.len(), 1); // One commit in test repo

        let start_dt =
            NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_dt =
            NaiveDateTime::parse_from_str("2023-01-10 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        assert!(timestamps[0] >= start_dt);
        assert!(timestamps[0] <= end_dt);
    }

    #[test]
    fn test_generate_timestamps_preserves_order() {
        let (_temp_dir, repo_path) = create_test_repo();
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-10 00:00:00".to_string()),
            show_history: false,
            pick_specific_commits: false,
            range: false,
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_ok());

        let timestamps = result.unwrap();

        // Check that timestamps are in ascending order
        for i in 1..timestamps.len() {
            assert!(timestamps[i] >= timestamps[i - 1]);
        }
    }
}
