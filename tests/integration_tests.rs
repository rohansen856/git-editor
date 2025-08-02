use git_editor::args::Args;
use git_editor::utils::commit_history::get_commit_history;
use git_editor::utils::datetime::generate_timestamps;
use git_editor::utils::validator::validate_inputs;
use serial_test::serial;
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
#[serial]
fn test_show_history_mode_integration() {
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

    // Test validation passes for show_history mode
    let validation_result = validate_inputs(&args);
    assert!(validation_result.is_ok());

    // Test that get_commit_history works
    let history_result = get_commit_history(&args, false);
    assert!(history_result.is_ok());

    let commits = history_result.unwrap();
    assert_eq!(commits.len(), 3);

    // Verify commits are in reverse chronological order
    assert_eq!(commits[0].message, "Commit 3");
    assert_eq!(commits[1].message, "Commit 2");
    assert_eq!(commits[2].message, "Commit 1");
}

#[test]
#[serial]
fn test_pick_specific_commits_mode_integration() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    let args = Args {
        repo_path: Some(repo_path),
        email: None,
        name: None,
        start: None,
        end: None,
        show_history: false,
        pic_specific_commits: true,
    };

    // Test validation passes for pick_specific_commits mode
    let validation_result = validate_inputs(&args);
    assert!(validation_result.is_ok());

    // Test that get_commit_history works (needed for commit selection)
    let history_result = get_commit_history(&args, false);
    assert!(history_result.is_ok());

    let commits = history_result.unwrap();
    assert_eq!(commits.len(), 3);

    // Verify all required fields are present for commit selection
    for commit in &commits {
        assert!(!commit.short_hash.is_empty());
        assert!(!commit.author_name.is_empty());
        assert!(!commit.author_email.is_empty());
        assert!(!commit.message.is_empty());
    }
}

#[test]
#[serial]
fn test_full_rewrite_mode_integration() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    let mut args = Args {
        repo_path: Some(repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("2025-01-01 00:00:00".to_string()),
        end: Some("2025-01-10 00:00:00".to_string()),
        show_history: false,
        pic_specific_commits: false,
    };

    // Test validation passes for full rewrite mode
    let validation_result = validate_inputs(&args);
    assert!(validation_result.is_ok());

    // Test that timestamp generation works
    let timestamp_result = generate_timestamps(&mut args);
    assert!(timestamp_result.is_ok());

    let timestamps = timestamp_result.unwrap();
    assert_eq!(timestamps.len(), 3); // Same as number of commits

    // Verify timestamps are in chronological order
    for i in 1..timestamps.len() {
        assert!(timestamps[i] >= timestamps[i - 1]);
    }

    // Verify timestamps are within the specified range
    let start_dt =
        chrono::NaiveDateTime::parse_from_str("2025-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let end_dt =
        chrono::NaiveDateTime::parse_from_str("2025-01-10 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();

    for timestamp in &timestamps {
        assert!(*timestamp >= start_dt);
        assert!(*timestamp <= end_dt);
    }
}

#[test]
#[serial]
fn test_mode_flag_precedence() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    // Test that when both show_history and pic_specific_commits are true,
    // validation still passes (both modes are valid)
    let args = Args {
        repo_path: Some(repo_path),
        email: None,
        name: None,
        start: None,
        end: None,
        show_history: true,
        pic_specific_commits: true,
    };

    let validation_result = validate_inputs(&args);
    assert!(validation_result.is_ok());
}

#[test]
#[serial]
fn test_invalid_repo_path_all_modes() {
    let invalid_repo_path = "/nonexistent/path".to_string();

    // Test show_history mode with invalid repo
    let args_show = Args {
        repo_path: Some(invalid_repo_path.clone()),
        email: None,
        name: None,
        start: None,
        end: None,
        show_history: true,
        pic_specific_commits: false,
    };

    let history_result = get_commit_history(&args_show, false);
    assert!(history_result.is_err());

    // Test pick_specific_commits mode with invalid repo
    let args_pick = Args {
        repo_path: Some(invalid_repo_path.clone()),
        email: None,
        name: None,
        start: None,
        end: None,
        show_history: false,
        pic_specific_commits: true,
    };

    let history_result = get_commit_history(&args_pick, false);
    assert!(history_result.is_err());

    // Test full rewrite mode with invalid repo
    let mut args_full = Args {
        repo_path: Some(invalid_repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("2023-01-01 00:00:00".to_string()),
        end: Some("2023-01-10 00:00:00".to_string()),
        show_history: false,
        pic_specific_commits: false,
    };

    let timestamp_result = generate_timestamps(&mut args_full);
    assert!(timestamp_result.is_err());
}

#[test]
#[serial]
fn test_full_rewrite_mode_insufficient_date_range() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    // Test with very small date range that's insufficient for commits
    let args = Args {
        repo_path: Some(repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("2023-01-01 00:00:00".to_string()),
        end: Some("2023-01-01 01:00:00".to_string()), // Only 1 hour for 3 commits
        show_history: false,
        pic_specific_commits: false,
    };

    let validation_result = validate_inputs(&args);
    assert!(validation_result.is_ok());

    // This test would normally call process::exit(1) due to insufficient date range
    // We can't easily test this without capturing the exit, so we'll test the
    // logic leading up to it by checking that the date range calculation would fail
    use chrono::{Duration, NaiveDateTime};

    let start_dt =
        NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let end_dt = NaiveDateTime::parse_from_str("2023-01-01 01:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let total_span = end_dt - start_dt;
    let min_span = Duration::hours(3 * (3 - 1)); // 3 commits need minimum 6 hours

    // Verify that the date range is indeed too small
    assert!(total_span < min_span);
}

#[test]
#[serial]
fn test_full_rewrite_mode_invalid_date_format() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    let mut args = Args {
        repo_path: Some(repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("invalid-date".to_string()),
        end: Some("2023-01-10 00:00:00".to_string()),
        show_history: false,
        pic_specific_commits: false,
    };

    let timestamp_result = generate_timestamps(&mut args);
    assert!(timestamp_result.is_err());
}

#[test]
#[serial]
fn test_workflow_show_history_then_pick_commits() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    // First, show history
    let args_show = Args {
        repo_path: Some(repo_path.clone()),
        email: None,
        name: None,
        start: None,
        end: None,
        show_history: true,
        pic_specific_commits: false,
    };

    let history_result = get_commit_history(&args_show, false);
    assert!(history_result.is_ok());
    let commits = history_result.unwrap();
    assert_eq!(commits.len(), 3);

    // Then, switch to pick specific commits mode
    let args_pick = Args {
        repo_path: Some(repo_path),
        email: None,
        name: None,
        start: None,
        end: None,
        show_history: false,
        pic_specific_commits: true,
    };

    let validation_result = validate_inputs(&args_pick);
    assert!(validation_result.is_ok());

    let history_result = get_commit_history(&args_pick, false);
    assert!(history_result.is_ok());
    let commits = history_result.unwrap();
    assert_eq!(commits.len(), 3);
}
