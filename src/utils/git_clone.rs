use crate::utils::types::Result;
use colored::Colorize;
use git2::build::RepoBuilder;
use git2::{Cred, CredentialType, FetchOptions, RemoteCallbacks, Repository};
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

/// Clones a Git repository to a temporary directory (deleted on drop) and returns it.
pub fn clone_repository(git_url: &str) -> Result<TempDir> {
    let temp_dir =
        TempDir::new().map_err(|e| format!("Failed to create temporary directory: {e}"))?;
    clone_repository_to(git_url, temp_dir.path())?;
    Ok(temp_dir)
}

/// Clones a Git repository into `dest`, which must not exist or be empty.
pub fn clone_repository_to(git_url: &str, dest: &std::path::Path) -> Result<()> {
    let shown = redact_url(git_url);
    if dest.exists() && dest.read_dir()?.next().is_some() {
        return Err(format!("Clone directory is not empty: {}", dest.display()).into());
    }
    crate::say!("{}", "🔄 Cloning repository...".cyan());
    crate::say!("{} {}", "Repository:".bold(), shown.yellow());

    clone_with_credentials(git_url, dest).map_err(|e| {
        let detail = e.to_string().replace(git_url, &shown);
        format!("Failed to clone repository '{shown}': {detail}")
    })?;

    crate::say!(
        "{} {}",
        "✓ Successfully cloned to:".green(),
        dest.display().to_string().cyan()
    );
    Ok(())
}

/// Clone using the same credential sources as `git`: ssh-agent for SSH URLs
/// and configured credential helpers for HTTP(S). Each source is tried once.
fn clone_with_credentials(
    url: &str,
    path: &std::path::Path,
) -> std::result::Result<Repository, git2::Error> {
    let config = git2::Config::open_default().ok();
    let mut tried_agent = false;
    let mut tried_helper = false;
    let mut tried_default = false;

    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(move |url, username, allowed| {
        if allowed.contains(CredentialType::USERNAME) {
            return Cred::username(username.unwrap_or("git"));
        }
        if allowed.contains(CredentialType::SSH_KEY) && !tried_agent {
            tried_agent = true;
            return Cred::ssh_key_from_agent(username.unwrap_or("git"));
        }
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) && !tried_helper {
            tried_helper = true;
            if let Some(config) = &config {
                return Cred::credential_helper(config, url, username);
            }
        }
        if allowed.contains(CredentialType::DEFAULT) && !tried_default {
            tried_default = true;
            return Cred::default();
        }
        Err(git2::Error::from_str(
            "authentication failed: no usable credentials from ssh-agent or git credential helpers",
        ))
    });

    let mut fetch = FetchOptions::new();
    fetch.remote_callbacks(callbacks);
    RepoBuilder::new().fetch_options(fetch).clone(url, path)
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
    #[cfg(unix)] // ':' is not allowed in Windows file names
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
