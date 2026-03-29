use crate::args::Args;
use crate::utils::types::Result;
use chrono::{Duration, NaiveDateTime};
use rand::Rng;
use uuid::Uuid;

pub fn generate_timestamps(args: &mut Args) -> Result<Vec<NaiveDateTime>> {
    let start_dt =
        NaiveDateTime::parse_from_str(args.start.as_ref().unwrap(), "%Y-%m-%d %H:%M:%S")?;
    let end_dt = NaiveDateTime::parse_from_str(args.end.as_ref().unwrap(), "%Y-%m-%d %H:%M:%S")?;

    if start_dt >= end_dt {
        return Err("Start datetime must be before end datetime".into());
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
            return Err("Failed to clone repository".into());
        }

        // Update repo_path to point to the cloned repository
        args.repo_path = Some(tmp_dir.to_string_lossy().to_string());
    }
    let total_commits = count_commits(args.repo_path.as_ref().unwrap())?;
    if total_commits == 0 {
        return Err("No commits found in repository".into());
    }

    let min_span = Duration::hours(3 * (total_commits as i64 - 1));
    let total_span = end_dt - start_dt;

    if total_span < min_span && !args.skip_range_check {
        return Err(format!(
            "Date range too small for {} commits. Need at least {} hours between start and end dates.\n\
            Tip: Pass --skip-range-check to skip this validation and distribute commits evenly across the given range.",
            total_commits,
            min_span.num_hours()
        ).into());
    }

    if total_span < min_span {
        // --skip-range-check: randomly distribute timestamps with 5-min minimum gap
        let mut timestamps = Vec::with_capacity(total_commits);
        if total_commits == 1 {
            timestamps.push(start_dt);
        } else {
            let min_gap = Duration::minutes(5);
            let min_total = min_gap * (total_commits as i32 - 1);
            if total_span < min_total {
                // Not enough room even for 5-min gaps - fall back to even spacing
                let step = total_span / (total_commits as i32 - 1);
                for i in 0..total_commits {
                    timestamps.push(start_dt + step * i as i32);
                }
            } else {
                // Random distribution with 5-min minimum gap
                let slack = total_span - min_total;
                let mut rng = rand::rng();
                let mut weights: Vec<f64> = (0..(total_commits - 1)).map(|_| rng.random()).collect();
                let sum: f64 = weights.iter().sum();
                for w in &mut weights {
                    *w = (*w / sum) * slack.num_seconds() as f64;
                }
                let mut current = start_dt;
                timestamps.push(current);
                for w in &weights {
                    let secs = w.round() as i64 + min_gap.num_seconds();
                    current += Duration::seconds(secs);
                    timestamps.push(current);
                }
            }
        }
        return Ok(timestamps);
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
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            skip_range_check: false,
            docs: false,
            _temp_dir: None,
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
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            skip_range_check: false,
            docs: false,
            _temp_dir: None,
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
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            skip_range_check: false,
            docs: false,
            _temp_dir: None,
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_ok());

        let timestamps = result.unwrap();

        // Check that timestamps are in ascending order
        for i in 1..timestamps.len() {
            assert!(timestamps[i] >= timestamps[i - 1]);
        }
    }

    fn create_test_repo_with_n_commits(n: usize) -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();
        let repo = git2::Repository::init(&repo_path).unwrap();

        for i in 1..=n {
            let file_path = temp_dir.path().join(format!("file{i}.txt"));
            fs::write(&file_path, format!("content {i}")).unwrap();

            let mut index = repo.index().unwrap();
            index
                .add_path(std::path::Path::new(&format!("file{i}.txt")))
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
    fn test_small_range_error_mentions_skip_flag() {
        let (_temp_dir, repo_path) = create_test_repo_with_n_commits(5);
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-01 01:00:00".to_string()), // 1 hour for 5 commits
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            skip_range_check: false,
            docs: false,
            _temp_dir: None,
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("--skip-range-check"),
            "Error message should mention --skip-range-check flag, got: {err_msg}"
        );
    }

    #[test]
    fn test_skip_range_check_produces_correct_count() {
        let (_temp_dir, repo_path) = create_test_repo_with_n_commits(5);
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-01 01:00:00".to_string()), // 1 hour for 5 commits
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            skip_range_check: true,
            docs: false,
            _temp_dir: None,
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_ok(), "skip_range_check should succeed: {:?}", result.err());

        let timestamps = result.unwrap();
        assert_eq!(timestamps.len(), 5);
    }

    #[test]
    fn test_skip_range_check_respects_5min_gap_and_bounds() {
        let (_temp_dir, repo_path) = create_test_repo_with_n_commits(3);
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-01 02:00:00".to_string()), // 2 hours for 3 commits (enough for 5-min gaps)
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            skip_range_check: true,
            docs: false,
            _temp_dir: None,
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_ok());

        let timestamps = result.unwrap();
        let start_dt =
            NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_dt =
            NaiveDateTime::parse_from_str("2023-01-01 02:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

        // All timestamps should be within bounds
        for ts in &timestamps {
            assert!(*ts >= start_dt, "Timestamp {ts} is before start");
            assert!(*ts <= end_dt, "Timestamp {ts} is after end");
        }

        // Each consecutive gap should be >= 5 minutes
        for i in 1..timestamps.len() {
            let gap = timestamps[i] - timestamps[i - 1];
            assert!(
                gap >= Duration::minutes(5),
                "Gap between timestamps[{i}] and [{prev}] is {gap_min} mins, expected >= 5",
                prev = i - 1,
                gap_min = gap.num_minutes()
            );
        }

        // Timestamps should be in ascending order
        for i in 1..timestamps.len() {
            assert!(timestamps[i] >= timestamps[i - 1]);
        }
    }

    #[test]
    fn test_skip_range_check_single_commit() {
        let (_temp_dir, repo_path) = create_test_repo(); // single commit
        let mut args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-01 00:01:00".to_string()), // 1 minute
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            skip_range_check: true,
            docs: false,
            _temp_dir: None,
        };

        let result = generate_timestamps(&mut args);
        assert!(result.is_ok());

        let timestamps = result.unwrap();
        assert_eq!(timestamps.len(), 1);
    }
}
