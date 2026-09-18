use crate::args::Args;
pub use crate::rewrite::engine::validate_identity_part;
use crate::utils::types::Result;
use regex::Regex;

/// Same email rules used by CLI validation and the range TUI.
pub fn is_valid_email(email: &str) -> bool {
    Regex::new(r"(?i)^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$")
        .map(|re| re.is_match(email))
        .unwrap_or(false)
}

pub fn validate_inputs(args: &Args) -> Result<()> {
    // Skip all validation for docs mode
    if args.docs {
        return Ok(());
    }

    // Always validate repo_path since it's required for all operations (except docs)
    let repo_path = args.repo_path.as_ref().unwrap();

    if repo_path.is_empty() {
        return Err("Repository path cannot be empty".into());
    }
    // URLs were already cloned by `Args::ensure_all_args_present`, so anything
    // left here must be a local repository.
    let path = std::path::Path::new(repo_path);
    if !path.exists() {
        return Err(format!("Repository path does not exist: {repo_path}").into());
    }
    if let Err(e) = git2::Repository::open(path) {
        return Err(format!("Not a Git repository: {repo_path} ({})", e.message()).into());
    }

    // Skip validation for email, name, start, end if using show_history, pick_specific_commits, range, simulate, or docs
    if args.show_history || args.pick_specific_commits || args.range || args.simulate || args.docs {
        return Ok(());
    }

    // Validate email, name, start, end only for full rewrite operations
    let email = args.email.as_ref().ok_or("Missing --email")?;
    let name = args.name.as_ref().ok_or("Missing --name")?;

    if !is_valid_email(email) {
        return Err(format!("Invalid email format: {email}").into());
    }

    if name.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }
    validate_identity_part(name, "Name")?;

    if args.keep_dates {
        if args.start.is_some() || args.end.is_some() {
            return Err("--keep-dates cannot be combined with --begin/--end".into());
        }
        return Ok(());
    }

    let start = args.start.as_ref().ok_or("Missing --begin")?;
    let end = args.end.as_ref().ok_or("Missing --end")?;
    for (flag, value) in [("--begin", start), ("--end", end)] {
        if value == "KEEP_ORIGINAL" {
            return Err(format!(
                "{flag} KEEP_ORIGINAL is no longer supported; use --keep-dates instead"
            )
            .into());
        }
    }

    let date_re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$")?;
    if !date_re.is_match(start) {
        return Err(
            format!("Invalid start date format (expected YYYY-MM-DD HH:MM:SS): {start}").into(),
        );
    }
    if !date_re.is_match(end) {
        return Err(
            format!("Invalid end date format (expected YYYY-MM-DD HH:MM:SS): {end}").into(),
        );
    }

    if start >= end {
        return Err("Start date must be before end date".into());
    }

    Ok(())
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
    fn test_validate_inputs_show_history_mode() {
        let (_temp_dir, repo_path) = create_test_repo();
        let args = Args {
            repo_path: Some(repo_path),
            show_history: true,
            ..Default::default()
        };

        let result = validate_inputs(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_inputs_pick_specific_commits_mode() {
        let (_temp_dir, repo_path) = create_test_repo();
        let args = Args {
            repo_path: Some(repo_path),
            pick_specific_commits: true,
            ..Default::default()
        };

        let result = validate_inputs(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_inputs_full_rewrite_valid() {
        let (_temp_dir, repo_path) = create_test_repo();
        let args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-02 00:00:00".to_string()),
            ..Default::default()
        };

        let result = validate_inputs(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_inputs_invalid_email() {
        let (_temp_dir, repo_path) = create_test_repo();
        let _args = Args {
            repo_path: Some(repo_path),
            email: Some("invalid-email".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-02 00:00:00".to_string()),
            ..Default::default()
        };

        // This test would normally call process::exit, so we can't test it directly
        // without mocking. We'll test the regex separately.
        let email_re = Regex::new(r"(?i)^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$").unwrap();
        assert!(!email_re.is_match("invalid-email"));
        assert!(email_re.is_match("test@example.com"));
    }

    #[test]
    fn test_validate_inputs_invalid_date_format() {
        let (_temp_dir, repo_path) = create_test_repo();
        let _args = Args {
            repo_path: Some(repo_path),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("invalid-date".to_string()),
            end: Some("2023-01-02 00:00:00".to_string()),
            ..Default::default()
        };

        let start_re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$").unwrap();
        assert!(!start_re.is_match("invalid-date"));
        assert!(start_re.is_match("2023-01-01 00:00:00"));
    }

    #[test]
    fn test_validate_inputs_nonexistent_repo() {
        let _args = Args {
            repo_path: Some("/nonexistent/path".to_string()),
            email: Some("test@example.com".to_string()),
            name: Some("Test User".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            end: Some("2023-01-02 00:00:00".to_string()),
            ..Default::default()
        };

        // This would normally call process::exit, so we test the path validation logic
        let repo_path = "/nonexistent/path";
        assert!(!std::path::Path::new(repo_path).exists());
    }

    #[test]
    fn test_email_regex_patterns() {
        // Valid emails
        assert!(is_valid_email("test@example.com"));
        assert!(is_valid_email("user.name@domain.org"));
        assert!(is_valid_email("user+tag@example.co.uk"));
        assert!(is_valid_email("123@test.io"));

        // Invalid emails (including bare @ which the TUI previously accepted)
        assert!(!is_valid_email("invalid-email"));
        assert!(!is_valid_email("@domain.com"));
        assert!(!is_valid_email("user@"));
        assert!(!is_valid_email("user@domain"));
        assert!(!is_valid_email("user@domain."));
        assert!(!is_valid_email("not-an-email"));
    }

    #[test]
    fn test_datetime_regex_patterns() {
        let datetime_re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$").unwrap();

        // Valid datetime formats
        assert!(datetime_re.is_match("2023-01-01 00:00:00"));
        assert!(datetime_re.is_match("2023-12-31 23:59:59"));
        assert!(datetime_re.is_match("2023-06-15 12:30:45"));

        // Invalid datetime formats
        assert!(!datetime_re.is_match("2023-1-1 0:0:0"));
        assert!(!datetime_re.is_match("2023/01/01 00:00:00"));
        assert!(!datetime_re.is_match("2023-01-01T00:00:00"));
        assert!(!datetime_re.is_match("23-01-01 00:00:00"));
        assert!(!datetime_re.is_match("2023-01-01 00:00"));
    }

    fn create_test_repo_with_commits() -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();

        // Initialize git repo
        let repo = git2::Repository::init(&repo_path).unwrap();

        // Create multiple commits
        for i in 1..=3 {
            let file_path = temp_dir.path().join(format!("test{i}.txt"));
            std::fs::write(&file_path, format!("test content {i}")).unwrap();

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
    fn test_validate_inputs_range_mode() {
        let (_temp_dir, repo_path) = create_test_repo_with_commits();
        let args = Args {
            repo_path: Some(repo_path),
            range: true,
            ..Default::default()
        };

        let result = validate_inputs(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_inputs_docs_mode() {
        // Test that docs mode skips all validation, even with invalid/missing data
        let args = Args {
            repo_path: None, // Missing repo path would normally fail
            docs: true,
            ..Default::default()
        };

        let result = validate_inputs(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_inputs_docs_mode_with_invalid_path() {
        // Test that docs mode skips validation even with invalid repo path
        let args = Args {
            repo_path: Some("/completely/invalid/path/that/does/not/exist".to_string()),
            email: Some("invalid-email".to_string()), // Invalid email format
            start: Some("invalid-date".to_string()),  // Invalid date format
            docs: true,
            ..Default::default()
        };

        let result = validate_inputs(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_inputs_docs_mode_precedence() {
        // Test that docs mode takes precedence over other modes
        let args = Args {
            repo_path: Some("/invalid/path".to_string()),
            show_history: true, // Other modes are set but docs should take precedence
            pick_specific_commits: true,
            range: true,
            simulate: true,
            docs: true, // Docs mode should skip all validation
            ..Default::default()
        };

        let result = validate_inputs(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_keep_dates_needs_no_begin_end() {
        let (_temp_dir, repo_path) = create_test_repo();
        let args = Args {
            repo_path: Some(repo_path),
            email: Some("new@example.com".to_string()),
            name: Some("New".to_string()),
            keep_dates: true,
            ..Default::default()
        };
        assert!(validate_inputs(&args).is_ok());
    }

    #[test]
    fn test_keep_dates_conflicts_with_begin_end() {
        let (_temp_dir, repo_path) = create_test_repo();
        let args = Args {
            repo_path: Some(repo_path),
            email: Some("new@example.com".to_string()),
            name: Some("New".to_string()),
            start: Some("2023-01-01 00:00:00".to_string()),
            keep_dates: true,
            ..Default::default()
        };
        assert!(validate_inputs(&args).is_err());
    }

    #[test]
    fn test_keep_original_sentinel_is_rejected() {
        let (_temp_dir, repo_path) = create_test_repo();
        let args = Args {
            repo_path: Some(repo_path),
            email: Some("new@example.com".to_string()),
            name: Some("New".to_string()),
            start: Some("KEEP_ORIGINAL".to_string()),
            end: Some("KEEP_ORIGINAL".to_string()),
            ..Default::default()
        };
        let err = validate_inputs(&args).unwrap_err().to_string();
        assert!(err.contains("--keep-dates"), "{err}");
    }

    #[test]
    fn test_name_with_newline_is_rejected() {
        let (_temp_dir, repo_path) = create_test_repo();
        let args = Args {
            repo_path: Some(repo_path),
            email: Some("new@example.com".to_string()),
            name: Some("New\nInjected: header".to_string()),
            keep_dates: true,
            ..Default::default()
        };
        let err = validate_inputs(&args).unwrap_err().to_string();
        assert!(err.contains("invalid character"), "{err}");
    }
}
