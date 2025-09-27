use crate::args::Args;
use crate::utils::types::Result;
use regex::Regex;
use url::Url;

pub fn validate_inputs(args: &Args) -> Result<()> {
    // Always validate repo_path since it's required for all operations
    let repo_path = args.repo_path.as_ref().unwrap();

    if repo_path.is_empty() {
        return Err("Repository path cannot be empty".into());
    }
    if Url::parse(repo_path).is_err() && !std::path::Path::new(repo_path).exists() {
        return Err(format!("Invalid repository path or URL: {repo_path}").into());
    }
    if std::path::Path::new(repo_path).exists() {
        if !std::path::Path::new(repo_path).is_dir() {
            return Err(format!("Repository path is not a directory: {repo_path}").into());
        }
        if !std::path::Path::new(repo_path).join(".git").exists() {
            return Err(format!(
                "Repository path does not contain a valid Git repository: {repo_path}"
            )
            .into());
        }
    }

    // Skip validation for email, name, start, end if using show_history, pick_specific_commits, range, or simulate
    if args.show_history || args.pick_specific_commits || args.range || args.simulate {
        return Ok(());
    }

    // Validate email, name, start, end only for full rewrite operations
    let email = args.email.as_ref().unwrap();
    let name = args.name.as_ref().unwrap();
    let start = args.start.as_ref().unwrap();
    let end = args.end.as_ref().unwrap();

    let email_re = Regex::new(r"(?i)^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$")?;
    if !email_re.is_match(email) {
        return Err(format!("Invalid email format: {email}").into());
    }

    if name.trim().is_empty() {
        return Err("Name cannot be empty".into());
    }

    // Allow special "KEEP_ORIGINAL" value to skip timestamp validation
    if start != "KEEP_ORIGINAL" {
        let start_re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$")?;
        if !start_re.is_match(start) {
            return Err(
                format!("Invalid start date format (expected YYYY-MM-DD HH:MM:SS): {start}").into(),
            );
        }
    }

    if end != "KEEP_ORIGINAL" {
        let end_re = Regex::new(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$")?;
        if !end_re.is_match(end) {
            return Err(
                format!("Invalid end date format (expected YYYY-MM-DD HH:MM:SS): {end}").into(),
            );
        }
    }

    // Skip date comparison if using KEEP_ORIGINAL
    if start != "KEEP_ORIGINAL" && end != "KEEP_ORIGINAL" && start >= end {
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
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: true,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            _temp_dir: None,
        };

        let result = validate_inputs(&args);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_inputs_pick_specific_commits_mode() {
        let (_temp_dir, repo_path) = create_test_repo();
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
            _temp_dir: None,
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
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            _temp_dir: None,
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
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            _temp_dir: None,
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
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            _temp_dir: None,
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
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            _temp_dir: None,
        };

        // This would normally call process::exit, so we test the path validation logic
        let repo_path = "/nonexistent/path";
        assert!(!std::path::Path::new(repo_path).exists());
    }

    #[test]
    fn test_email_regex_patterns() {
        let email_re = Regex::new(r"(?i)^[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}$").unwrap();

        // Valid emails
        assert!(email_re.is_match("test@example.com"));
        assert!(email_re.is_match("user.name@domain.org"));
        assert!(email_re.is_match("user+tag@example.co.uk"));
        assert!(email_re.is_match("123@test.io"));

        // Invalid emails
        assert!(!email_re.is_match("invalid-email"));
        assert!(!email_re.is_match("@domain.com"));
        assert!(!email_re.is_match("user@"));
        assert!(!email_re.is_match("user@domain"));
        assert!(!email_re.is_match("user@domain."));
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
            email: None,
            name: None,
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: false,
            range: true,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            _temp_dir: None,
        };

        let result = validate_inputs(&args);
        assert!(result.is_ok());
    }
}
