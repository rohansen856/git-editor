//! Default identity lookup for prompts.
//!
//! Uses libgit2's configuration stack with the usual precedence: the
//! repository's `.git/config`, then `~/.gitconfig` / `$XDG_CONFIG_HOME/git/config`,
//! then the system config, including `[include]` files. Environment overrides
//! that only the `git` CLI understands (`GIT_CONFIG_GLOBAL`, `GIT_CONFIG_COUNT`,
//! …) are not applied.

use git2::{Config, Repository};

fn config_for(repo_path: Option<&str>) -> Option<Config> {
    repo_path
        .and_then(|path| Repository::open(path).ok())
        .and_then(|repo| repo.config().ok())
        .or_else(|| Config::open_default().ok())
}

fn read_value(repo_path: Option<&str>, key: &str) -> Option<String> {
    let config = config_for(repo_path)?.snapshot().ok()?;
    let value = config.get_string(key).ok()?;
    let value = value.trim().to_string();
    (!value.is_empty()).then_some(value)
}

/// `user.name` as Git would resolve it for `repo_path` (or globally).
pub fn get_git_user_name(repo_path: Option<&str>) -> Option<String> {
    read_value(repo_path, "user.name")
}

/// `user.email` as Git would resolve it for `repo_path` (or globally).
pub fn get_git_user_email(repo_path: Option<&str>) -> Option<String> {
    read_value(repo_path, "user.email")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn repository_config_takes_precedence() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Repo Local").unwrap();
        config.set_str("user.email", "local@example.com").unwrap();

        let path = dir.path().to_str();
        assert_eq!(get_git_user_name(path).as_deref(), Some("Repo Local"));
        assert_eq!(
            get_git_user_email(path).as_deref(),
            Some("local@example.com")
        );
    }

    #[test]
    fn blank_values_are_ignored() {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        repo.config().unwrap().set_str("user.name", "   ").unwrap();
        // Falls through to whatever the global config says, but never "   ".
        assert_ne!(
            get_git_user_name(dir.path().to_str()).as_deref(),
            Some("   ")
        );
    }

    #[test]
    fn missing_repository_falls_back_to_default_config() {
        // Must not panic or error for a path that is not a repository.
        let _ = get_git_user_name(Some("/definitely/not/a/repo"));
    }
}
