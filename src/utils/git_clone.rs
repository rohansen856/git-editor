use crate::utils::types::Result;
use colored::Colorize;
use git2::Repository;
use tempfile::TempDir;
use url::Url;

/// Checks if a string is a valid Git URL
pub fn is_git_url(input: &str) -> bool {
    if let Ok(url) = Url::parse(input) {
        match url.scheme() {
            "http" | "https" | "git" | "ssh" => true,
            _ => false,
        }
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

/// Clones a Git repository to a temporary directory and returns the path
pub fn clone_repository(git_url: &str) -> Result<TempDir> {
    println!("{}", "🔄 Cloning repository...".cyan());
    println!("{} {}", "Repository:".bold(), git_url.yellow());

    // Create a temporary directory
    let temp_dir = TempDir::new()
        .map_err(|e| format!("Failed to create temporary directory: {}", e))?;

    let repo_path = temp_dir.path();

    // Clone the repository
    let _repo = Repository::clone(git_url, repo_path)
        .map_err(|e| format!("Failed to clone repository '{}': {}", git_url, e))?;

    println!("{} {}", "✓ Successfully cloned to:".green(), repo_path.display().to_string().cyan());

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
                return format!("{}/{}", segments[segments.len() - 2], segments[segments.len() - 1]);
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
    normalized.split('/').last().unwrap_or("repository").to_string()
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