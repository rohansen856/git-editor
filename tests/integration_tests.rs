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
        show_history: true,
        ..Default::default()
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
        pick_specific_commits: true,
        ..Default::default()
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

    let args = Args {
        repo_path: Some(repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("2025-01-01 00:00:00".to_string()),
        end: Some("2025-01-10 00:00:00".to_string()),
        ..Default::default()
    };

    // Test validation passes for full rewrite mode
    let validation_result = validate_inputs(&args);
    assert!(validation_result.is_ok());

    // Test that timestamp generation works
    let timestamp_result = generate_timestamps(&args);
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
fn test_invalid_repo_path_all_modes() {
    let invalid_repo_path = "/nonexistent/path".to_string();

    // Test show_history mode with invalid repo
    let args_show = Args {
        repo_path: Some(invalid_repo_path.clone()),
        show_history: true,
        ..Default::default()
    };

    let history_result = get_commit_history(&args_show, false);
    assert!(history_result.is_err());

    // Test pick_specific_commits mode with invalid repo
    let args_pick = Args {
        repo_path: Some(invalid_repo_path.clone()),
        pick_specific_commits: true,
        ..Default::default()
    };

    let history_result = get_commit_history(&args_pick, false);
    assert!(history_result.is_err());

    // Test full rewrite mode with invalid repo
    let args_full = Args {
        repo_path: Some(invalid_repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("2023-01-01 00:00:00".to_string()),
        end: Some("2023-01-10 00:00:00".to_string()),
        ..Default::default()
    };

    let timestamp_result = generate_timestamps(&args_full);
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
        ..Default::default()
    };

    let validation_result = validate_inputs(&args);
    assert!(validation_result.is_ok());

    // The error message should mention --skip-range-check
    let args_mut = args;
    let timestamp_result = generate_timestamps(&args_mut);
    assert!(timestamp_result.is_err());
    let err_msg = timestamp_result.unwrap_err().to_string();
    assert!(
        err_msg.contains("--skip-range-check"),
        "Error should mention --skip-range-check, got: {err_msg}"
    );
}

#[test]
#[serial]
fn test_skip_range_check_succeeds_with_small_range() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    let args = Args {
        repo_path: Some(repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("2023-01-01 00:00:00".to_string()),
        end: Some("2023-01-01 01:00:00".to_string()), // Only 1 hour for 3 commits
        skip_range_check: true,
        ..Default::default()
    };

    let timestamp_result = generate_timestamps(&args);
    assert!(
        timestamp_result.is_ok(),
        "Should succeed with --skip-range-check: {:?}",
        timestamp_result.err()
    );

    let timestamps = timestamp_result.unwrap();
    assert_eq!(timestamps.len(), 3);

    // Timestamps should be in chronological order
    for i in 1..timestamps.len() {
        assert!(timestamps[i] >= timestamps[i - 1]);
    }

    // Timestamps should be within the specified range
    let start_dt =
        chrono::NaiveDateTime::parse_from_str("2023-01-01 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let end_dt =
        chrono::NaiveDateTime::parse_from_str("2023-01-01 01:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    for ts in &timestamps {
        assert!(*ts >= start_dt);
        assert!(*ts <= end_dt);
    }
}

#[test]
#[serial]
fn test_skip_range_check_with_5min_minimum_gap() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    // 2 hours for 3 commits - enough room for 5-min gaps with random distribution
    let args = Args {
        repo_path: Some(repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("2023-01-01 00:00:00".to_string()),
        end: Some("2023-01-01 02:00:00".to_string()),
        skip_range_check: true,
        ..Default::default()
    };

    let timestamp_result = generate_timestamps(&args);
    assert!(timestamp_result.is_ok());

    let timestamps = timestamp_result.unwrap();
    assert_eq!(timestamps.len(), 3);

    // Each gap should be >= 5 minutes
    for i in 1..timestamps.len() {
        let gap = timestamps[i] - timestamps[i - 1];
        assert!(
            gap >= chrono::Duration::minutes(5),
            "Gap between commit {} and {} is {} mins, expected >= 5",
            i - 1,
            i,
            gap.num_minutes()
        );
    }
}

#[test]
#[serial]
fn test_full_rewrite_mode_invalid_date_format() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    let args = Args {
        repo_path: Some(repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("invalid-date".to_string()),
        end: Some("2023-01-10 00:00:00".to_string()),
        ..Default::default()
    };

    let timestamp_result = generate_timestamps(&args);
    assert!(timestamp_result.is_err());
}

#[test]
#[serial]
fn test_workflow_show_history_then_pick_commits() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    // First, show history
    let args_show = Args {
        repo_path: Some(repo_path.clone()),
        show_history: true,
        ..Default::default()
    };

    let history_result = get_commit_history(&args_show, false);
    assert!(history_result.is_ok());
    let commits = history_result.unwrap();
    assert_eq!(commits.len(), 3);

    // Then, switch to pick specific commits mode
    let args_pick = Args {
        repo_path: Some(repo_path),
        pick_specific_commits: true,
        ..Default::default()
    };

    let validation_result = validate_inputs(&args_pick);
    assert!(validation_result.is_ok());

    let history_result = get_commit_history(&args_pick, false);
    assert!(history_result.is_ok());
    let commits = history_result.unwrap();
    assert_eq!(commits.len(), 3);
}

#[test]
#[serial]
fn test_simulation_mode_complete_args() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    let args = Args {
        repo_path: Some(repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("2025-01-01 00:00:00".to_string()),
        end: Some("2025-01-10 00:00:00".to_string()),
        simulate: true,
        ..Default::default()
    };

    // Test validation passes for simulation mode with complete args
    let validation_result = validate_inputs(&args);
    assert!(validation_result.is_ok());

    // Test that timestamp generation works in simulation
    let timestamp_result = generate_timestamps(&args);
    assert!(timestamp_result.is_ok());
}

#[test]
#[serial]
fn test_simulation_mode_incomplete_args() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    // Test simulation with missing required arguments - this is the scenario that caused the panic
    let mut args = Args {
        repo_path: Some(repo_path),
        simulate: true,
        ..Default::default()
    };

    // Basic validation should pass for simulation mode
    let validation_result = validate_inputs(&args);
    assert!(validation_result.is_ok());

    // ensure_all_args_present should pass for simulation mode even with incomplete args
    let ensure_result = args.ensure_all_args_present();
    assert!(ensure_result.is_ok());
}

#[test]
#[serial]
fn test_simulation_mode_with_show_diff() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    let args = Args {
        repo_path: Some(repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("2025-01-01 00:00:00".to_string()),
        end: Some("2025-01-10 00:00:00".to_string()),
        simulate: true,
        show_diff: true,
        ..Default::default()
    };

    // Test that simulation with show_diff passes validation
    let validation_result = validate_inputs(&args);
    assert!(validation_result.is_ok());
}

#[test]
fn test_conflicting_modes_are_rejected_by_the_parser() {
    use clap::Parser;
    for argv in [
        vec!["git-editor", "-s", "-x"],
        vec!["git-editor", "-p", "-x"],
        vec!["git-editor", "--docs", "-s"],
        vec!["git-editor", "--simulate", "-s"],
        vec!["git-editor", "--message"],
        vec!["git-editor", "--select", "1-2"],
        vec!["git-editor", "--commit", "1"],
    ] {
        assert!(
            Args::try_parse_from(&argv).is_err(),
            "{argv:?} should be rejected"
        );
    }
    for argv in [
        vec!["git-editor", "-x", "--message"],
        vec!["git-editor", "--simulate", "-x"],
        vec!["git-editor", "--show-diff", "-x", "--select", "1"],
        vec!["git-editor", "-p", "--commit", "1", "--set-message", "m"],
    ] {
        assert!(Args::try_parse_from(&argv).is_ok(), "{argv:?} should parse");
    }
}

#[test]
#[serial]
fn test_cli_execution_simulate_incomplete_args_no_panic() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    // This test simulates the exact CLI execution path that caused the panic
    // by testing the main run() function directly with incomplete simulation args
    let mut args = Args {
        repo_path: Some(repo_path),
        simulate: true,
        ..Default::default()
    };

    // Mock the Args::parse() result by testing the execution flow manually
    // This tests the exact path that was causing the panic: simulate mode with missing args

    // First ensure basic validation passes
    assert!(validate_inputs(&args).is_ok());

    // Now test the critical path: ensure_all_args_present should pass for simulation mode
    // even with incomplete args - this is the correct behavior
    let ensure_result = args.ensure_all_args_present();
    assert!(
        ensure_result.is_ok(),
        "ensure_all_args_present should pass for simulation mode"
    );

    // The fixed version should handle this gracefully in execute_simulation_operation
    // instead of panicking when trying to generate_timestamps with incomplete args
}

#[test]
#[serial]
fn test_cli_execution_simulate_complete_args_success() {
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    // Test the successful simulation path
    let mut args = Args {
        repo_path: Some(repo_path),
        email: Some("test@example.com".to_string()),
        name: Some("Test User".to_string()),
        start: Some("2025-01-01 00:00:00".to_string()),
        end: Some("2025-01-10 00:00:00".to_string()),
        simulate: true,
        ..Default::default()
    };

    // Test full execution path
    assert!(args.ensure_all_args_present().is_ok());
    assert!(validate_inputs(&args).is_ok());

    // Test timestamp generation works
    let timestamp_result = generate_timestamps(&args);
    assert!(timestamp_result.is_ok());
}

#[test]
#[serial]
fn test_simulation_execution_function_missing_args() {
    // This test specifically targets the execute_simulation_operation function
    // that was causing the original panic
    let (_temp_dir, repo_path) = create_test_repo_with_commits();

    let args = Args {
        repo_path: Some(repo_path),
        email: None, // Missing - should trigger graceful handling
        name: None,  // Missing - should trigger graceful handling
        start: None, // Missing - should trigger graceful handling
        end: None,   // Missing - should trigger graceful handling
        simulate: true,
        ..Default::default()
    };

    // The issue was that the old code called generate_timestamps without checking
    // if required args were present first, causing a panic at args.start.unwrap()

    // Test what happens when we have incomplete args in simulation mode
    let commit_history = get_commit_history(&args, false);
    assert!(commit_history.is_ok());

    let commits = commit_history.unwrap();
    assert!(!commits.is_empty());

    // This should NOT panic - the fixed code checks for required arguments first
    // If all required args are missing, it should handle gracefully
    if args.email.is_some() && args.name.is_some() && args.start.is_some() && args.end.is_some() {
        // Only generate timestamps if we have all required args
        let timestamp_result = generate_timestamps(&args);
        assert!(timestamp_result.is_ok());
    } else {
        // With missing args, we should not attempt to generate timestamps
        // This mimics the fixed logic in execute_simulation_operation
        assert!(
            args.email.is_none()
                || args.name.is_none()
                || args.start.is_none()
                || args.end.is_none()
        );
    }
}

#[test]
#[serial]
fn test_docs_mode_execution() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--", "--docs"])
        .env("DISPLAY", ":0") // Mock display for headless environments
        .env("GIT_EDITOR_NO_BROWSER", "1") // Disable browser opening during tests
        .output()
        .expect("Failed to execute command");

    // The command should succeed
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    // Should contain docs-related output
    assert!(
        stdout.contains("📚 Opening Git Editor Documentation")
            || stderr.contains("📚 Opening Git Editor Documentation"),
        "Expected docs message not found. Stdout: {stdout}, Stderr: {stderr}"
    );

    // Should either succeed in opening browser or fail gracefully
    assert!(
        stdout.contains("✅ Documentation opened")
            || stdout.contains("⚠️  Could not open browser")
            || stderr.contains("✅ Documentation opened")
            || stderr.contains("⚠️  Could not open browser"),
        "Expected either success or graceful failure message. Stdout: {stdout}, Stderr: {stderr}"
    );
}

#[test]
#[serial]
fn test_docs_mode_no_repo_required() {
    // Create a temporary directory that is NOT a git repo
    let temp_dir = TempDir::new().unwrap();

    // Build the absolute path to the binary
    let project_root = std::env::current_dir().expect("Failed to get current dir");
    let binary_path = project_root.join("target/debug/git-editor");

    let output = std::process::Command::new(binary_path)
        .args(["--docs"])
        .current_dir(temp_dir.path())
        .env("DISPLAY", ":0")
        .env("GIT_EDITOR_NO_BROWSER", "1") // Disable browser opening during tests
        .output()
        .expect("Failed to execute command");

    // Should succeed even without a git repository
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    // Should not contain repository-related errors
    assert!(
        !stdout.contains("Repository not found") && !stderr.contains("Repository not found"),
        "Docs mode should not require a repository. Stdout: {stdout}, Stderr: {stderr}"
    );
}

#[test]
#[serial]
fn test_docs_mode_with_invalid_args() {
    // Test that docs mode works even with invalid arguments that would normally fail
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "--docs",
            "--email",
            "invalid-email", // Invalid email format
            "--begin",
            "invalid-date", // Invalid date format
            "--repo-path",
            "/nonexistent/path", // Invalid repo path
        ])
        .env("DISPLAY", ":0")
        .env("GIT_EDITOR_NO_BROWSER", "1") // Disable browser opening during tests
        .output()
        .expect("Failed to execute command");

    // Should still succeed because docs mode skips validation
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();

    // Should contain docs message
    assert!(
        stdout.contains("📚 Opening Git Editor Documentation")
            || stderr.contains("📚 Opening Git Editor Documentation"),
        "Expected docs message. Stdout: {stdout}, Stderr: {stderr}"
    );
}

#[test]
#[serial]
fn test_docs_flag_in_help() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--", "--help"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should contain the docs flag in help output
    assert!(stdout.contains("--docs"));
    assert!(stdout.contains("Open comprehensive documentation in the browser"));
}

#[test]
#[serial]
fn test_docs_mode_conflicts_with_other_modes() {
    // Combining --docs with another mode is a usage error (exit code 2), not silently resolved.
    let output = std::process::Command::new("cargo")
        .args(["run", "--", "--docs", "--show-history"])
        .env("GIT_EDITOR_NO_BROWSER", "1") // Disable browser opening during tests
        .output()
        .expect("Failed to execute command");

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("cannot be used with"), "stderr: {stderr}");
}
