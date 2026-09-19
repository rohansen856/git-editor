use crate::utils::types::Result;
use colored::Colorize;
use git2::Repository;
use tempfile::TempDir;
use url::Url;

/// Checks if a string is a valid Git URL
pub fn is_git_url(input: &str) -> bool {
    // An existing local path is never a URL, even if it contains '@' and ':'.
    if std::path::Path::new(input).exists() {
        return false;
    }
    if let Ok(url) = Url::parse(input) {
        matches!(url.scheme(), "http" | "https" | "git" | "ssh")
    } else {
        // Check for SSH format like git@github.com:user/repo.git
        input.contains('@') && input.contains(':') && !input.contains(' ')
    }
}

/// Normalizes a Git URL by removing .git suffix if present
pub fn normalize_git_url(url: &str) -> String {
    if url.ends_with(".git") {
        url.trim_end_matches(".git").to_string()
    } else {
        url.to_string()
    }
}

/// Hide credentials embedded in a URL (`https://user:token@host` → `https://user:***@host`).
pub fn redact_url(input: &str) -> String {
    let Ok(mut url) = Url::parse(input) else {
        return input.to_string();
    };
    if url.password().is_some() {
        let _ = url.set_password(Some("***"));
    } else if !url.username().is_empty() && matches!(url.scheme(), "http" | "https") {
        // A bare username on HTTP(S) is usually a token.
        let _ = url.set_username("***");
    } else {
        return input.to_string();
    }
    url.to_string()
}

/// Clones a Git repository to a temporary directory and returns the path
pub fn clone_repository(git_url: &str) -> Result<TempDir> {
    let shown = redact_url(git_url);
    println!("{}", "🔄 Cloning repository...".cyan());
    println!("{} {}", "Repository:".bold(), shown.yellow());

    // Create a temporary directory
    let temp_dir =
        TempDir::new().map_err(|e| format!("Failed to create temporary directory: {e}"))?;

    let repo_path = temp_dir.path();

    // Clone the repository
    let _repo = Repository::clone(git_url, repo_path).map_err(|e| {
        let detail = e.to_string().replace(git_url, &shown);
        format!("Failed to clone repository '{shown}': {detail}")
    })?;

    println!(
        "{} {}",
        "✓ Successfully cloned to:".green(),
        repo_path.display().to_string().cyan()
    );

    Ok(temp_dir)
}

/// Gets repository name from Git URL for display purposes
pub fn get_repo_name_from_url(git_url: &str) -> String {
    let normalized = normalize_git_url(git_url);

    if let Ok(url) = Url::parse(&normalized) {
        // Extract from path like /user/repo
        if let Some(segments) = url.path_segments() {
            let segments: Vec<&str> = segments.collect();
            if segments.len() >= 2 {
                return format!(
                    "{}/{}",
                    segments[segments.len() - 2],
                    segments[segments.len() - 1]
                );
            } else if segments.len() == 1 {
                return segments[0].to_string();
            }
        }
        return url.path().trim_start_matches('/').to_string();
    }

    // Handle SSH format like git@github.com:user/repo
    if let Some(colon_pos) = normalized.rfind(':') {
        let path_part = &normalized[colon_pos + 1..];
        return path_part.to_string();
    }

    // Fallback: use the last part of the URL
    normalized
        .split('/')
        .next_back()
        .unwrap_or("repository")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_url() {
        assert!(is_git_url("https://github.com/user/repo"));
        assert!(is_git_url("https://github.com/user/repo.git"));
        assert!(is_git_url("http://gitlab.com/user/repo"));
        assert!(is_git_url("git://github.com/user/repo.git"));
        assert!(is_git_url("git@github.com:user/repo.git"));

        assert!(!is_git_url("./local/path"));
        assert!(!is_git_url("/absolute/path"));
        assert!(!is_git_url("not-a-url"));
        assert!(!is_git_url("file:///local/path"));
    }

    #[test]
    fn test_existing_path_with_at_and_colon_is_local() {
        let dir = tempfile::TempDir::new().unwrap();
        let odd = dir.path().join("we@ird:dir");
        std::fs::create_dir(&odd).unwrap();
        assert!(!is_git_url(odd.to_str().unwrap()));
    }

    #[test]
    fn test_redact_url_hides_credentials() {
        assert_eq!(
            redact_url("https://user:ghp_secret@github.com/o/r.git"),
            "https://user:***@github.com/o/r.git"
        );
        assert_eq!(
            redact_url("https://ghp_secret@github.com/o/r.git"),
            "https://***@github.com/o/r.git"
        );
        assert_eq!(
            redact_url("https://github.com/o/r.git"),
            "https://github.com/o/r.git"
        );
        assert_eq!(
            redact_url("git@github.com:o/r.git"),
            "git@github.com:o/r.git"
        );
        assert_eq!(redact_url("ssh://git@host/r.git"), "ssh://git@host/r.git");
    }

    #[test]
    fn test_clone_error_does_not_leak_token() {
        let err = clone_repository("https://u:ghp_TOPSECRET@127.0.0.1:9/o/r.git")
            .unwrap_err()
            .to_string();
        assert!(!err.contains("TOPSECRET"), "{err}");
    }

    #[test]
    fn test_normalize_git_url() {
        assert_eq!(
            normalize_git_url("https://github.com/user/repo.git"),
            "https://github.com/user/repo"
        );
        assert_eq!(
            normalize_git_url("https://github.com/user/repo"),
            "https://github.com/user/repo"
        );
        assert_eq!(
            normalize_git_url("git@github.com:user/repo.git"),
            "git@github.com:user/repo"
        );
    }

    #[test]
    fn test_get_repo_name_from_url() {
        assert_eq!(
            get_repo_name_from_url("https://github.com/rohansen856/git-editor.git"),
            "rohansen856/git-editor"
        );
        assert_eq!(
            get_repo_name_from_url("https://github.com/user/repo"),
            "user/repo"
        );
        assert_eq!(
            get_repo_name_from_url("git@github.com:user/repo.git"),
            "user/repo"
        );
        assert_eq!(
            get_repo_name_from_url("https://gitlab.com/namespace/project"),
            "namespace/project"
        );
    }
}
