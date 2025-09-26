use std::path::PathBuf;
use std::process::Command;

// Attempts to get git configuration values for user name and email. First tries the git command, then falls back to reading ~/.gitconfig directly.
pub fn get_git_user_name() -> Option<String> {
    // Try git command first
    if let Ok(output) = Command::new("git")
        .args(["config", "--global", "user.name"])
        .output()
    {
        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }
    }

    // Fallback to reading ~/.gitconfig file
    read_gitconfig_value("user", "name")
}

// Attempts to get git configuration values for user email. First tries the git command, then falls back to reading ~/.gitconfig directly.
pub fn get_git_user_email() -> Option<String> {
    // Try git command first
    if let Ok(output) = Command::new("git")
        .args(["config", "--global", "user.email"])
        .output()
    {
        if output.status.success() {
            let email = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !email.is_empty() {
                return Some(email);
            }
        }
    }

    // Fallback to reading ~/.gitconfig file
    read_gitconfig_value("user", "email")
}

// Reads a specific value from the git config file directly. This is used as a fallback when the git command is not available. Handles cross-platform git config locations.
fn read_gitconfig_value(section: &str, key: &str) -> Option<String> {
    use std::fs;

    // Get the appropriate git config path for the current OS
    let gitconfig_paths = get_gitconfig_paths();

    // Try each possible gitconfig path
    for gitconfig_path in gitconfig_paths {
        if let Ok(content) = fs::read_to_string(&gitconfig_path) {
            if let Some(value) = parse_gitconfig(&content, section, key) {
                return Some(value);
            }
        }
    }

    None
}

// Returns the possible git config file paths for the current operating system. Returns them in order of precedence (user config first, then system config).
fn get_gitconfig_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // User-level git config (highest precedence)
    if let Some(user_config) = get_user_gitconfig_path() {
        paths.push(user_config);
    }

    // System-level git config (lower precedence)
    if let Some(system_config) = get_system_gitconfig_path() {
        paths.push(system_config);
    }

    paths
}

// Gets the user-level git config path for the current OS.
fn get_user_gitconfig_path() -> Option<PathBuf> {
    // Try different environment variables for home directory
    let home_dir = if let Ok(home) = std::env::var("HOME") {
        // Unix-like systems (Linux, macOS)
        PathBuf::from(home)
    } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
        // Windows
        PathBuf::from(userprofile)
    } else if let Ok(homedrive) = std::env::var("HOMEDRIVE") {
        if let Ok(homepath) = std::env::var("HOMEPATH") {
            // Alternative Windows approach
            PathBuf::from(homedrive).join(homepath)
        } else {
            return None;
        }
    } else {
        return None;
    };

    Some(home_dir.join(".gitconfig"))
}

// Gets the system-level git config path for the current OS.
fn get_system_gitconfig_path() -> Option<PathBuf> {
    if cfg!(windows) {
        // Windows: Try common Git installation locations
        let possible_paths = vec![
            PathBuf::from(r"C:\ProgramData\Git\config"),
            PathBuf::from(r"C:\Program Files\Git\etc\gitconfig"),
            PathBuf::from(r"C:\Program Files (x86)\Git\etc\gitconfig"),
        ];

        // Return the first one that exists
        for path in possible_paths {
            if path.exists() {
                return Some(path);
            }
        }
        None
    } else {
        // Unix-like systems (Linux, macOS)
        Some(PathBuf::from("/etc/gitconfig"))
    }
}

// Simple parser for .gitconfig files to extract specific values. Handles basic INI-style format with [section] and key = value pairs.
fn parse_gitconfig(content: &str, target_section: &str, target_key: &str) -> Option<String> {
    let mut in_target_section = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Check for section headers
        if line.starts_with('[') && line.ends_with(']') {
            let section = &line[1..line.len() - 1];
            in_target_section = section.trim().eq_ignore_ascii_case(target_section);
            continue;
        }

        // If we're in the target section, look for the key
        if in_target_section {
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                if key.eq_ignore_ascii_case(target_key) {
                    let value = line[eq_pos + 1..].trim();
                    // Remove quotes if present
                    let value = if (value.starts_with('"') && value.ends_with('"'))
                        || (value.starts_with('\'') && value.ends_with('\''))
                    {
                        &value[1..value.len() - 1]
                    } else {
                        value
                    };
                    return Some(value.to_string());
                }
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gitconfig_basic() {
        let config = r#"
[user]
    name = John Doe
    email = john@example.com

[core]
    editor = vim
"#;

        assert_eq!(
            parse_gitconfig(config, "user", "name"),
            Some("John Doe".to_string())
        );
        assert_eq!(
            parse_gitconfig(config, "user", "email"),
            Some("john@example.com".to_string())
        );
        assert_eq!(
            parse_gitconfig(config, "core", "editor"),
            Some("vim".to_string())
        );
        assert_eq!(parse_gitconfig(config, "user", "nonexistent"), None);
        assert_eq!(parse_gitconfig(config, "nonexistent", "name"), None);
    }

    #[test]
    fn test_parse_gitconfig_with_quotes() {
        let config = r#"
[user]
    name = "John Doe"
    email = 'john@example.com'
"#;

        assert_eq!(
            parse_gitconfig(config, "user", "name"),
            Some("John Doe".to_string())
        );
        assert_eq!(
            parse_gitconfig(config, "user", "email"),
            Some("john@example.com".to_string())
        );
    }

    #[test]
    fn test_parse_gitconfig_case_insensitive() {
        let config = r#"
[USER]
    NAME = John Doe
    EMAIL = john@example.com
"#;

        assert_eq!(
            parse_gitconfig(config, "user", "name"),
            Some("John Doe".to_string())
        );
        assert_eq!(
            parse_gitconfig(config, "user", "email"),
            Some("john@example.com".to_string())
        );
    }

    #[test]
    fn test_parse_gitconfig_with_comments() {
        let config = r#"
# This is a comment
[user]
    name = John Doe  # inline comment
    ; This is also a comment
    email = john@example.com
"#;

        assert_eq!(
            parse_gitconfig(config, "user", "name"),
            Some("John Doe  # inline comment".to_string())
        );
        assert_eq!(
            parse_gitconfig(config, "user", "email"),
            Some("john@example.com".to_string())
        );
    }

    #[test]
    fn test_parse_gitconfig_empty_values() {
        let config = r#"
[user]
    name =
    email = john@example.com
"#;

        assert_eq!(
            parse_gitconfig(config, "user", "name"),
            Some("".to_string())
        );
        assert_eq!(
            parse_gitconfig(config, "user", "email"),
            Some("john@example.com".to_string())
        );
    }

    #[test]
    fn test_get_git_user_functions_exist() {
        // These functions should not panic and should return Option values
        let _name = get_git_user_name();
        let _email = get_git_user_email();
    }

    #[test]
    fn test_get_user_gitconfig_path() {
        // Test that the function returns a path
        let path = get_user_gitconfig_path();

        // Should return Some path on all supported platforms
        assert!(path.is_some(), "Should return a gitconfig path");

        let path = path.unwrap();
        assert!(
            path.ends_with(".gitconfig"),
            "Path should end with .gitconfig"
        );
    }

    #[test]
    fn test_get_gitconfig_paths() {
        // Test that we get at least one path back
        let paths = get_gitconfig_paths();
        assert!(
            !paths.is_empty(),
            "Should return at least one gitconfig path"
        );

        // First path should be the user config
        assert!(
            paths[0].ends_with(".gitconfig"),
            "First path should be user config"
        );
    }

    #[test]
    fn test_cross_platform_home_detection() {
        // This test verifies that we can detect home directory on different platforms
        use std::env;

        // Save original env vars
        let original_home = env::var("HOME").ok();
        let original_userprofile = env::var("USERPROFILE").ok();
        let original_homedrive = env::var("HOMEDRIVE").ok();
        let original_homepath = env::var("HOMEPATH").ok();

        // Test Unix-like behavior (HOME env var)
        env::remove_var("USERPROFILE");
        env::remove_var("HOMEDRIVE");
        env::remove_var("HOMEPATH");
        env::set_var("HOME", "/tmp/test-home");

        let path = get_user_gitconfig_path();
        assert!(path.is_some());
        assert_eq!(path.unwrap().to_string_lossy(), "/tmp/test-home/.gitconfig");

        // Test Windows behavior (USERPROFILE env var)
        env::remove_var("HOME");
        env::set_var("USERPROFILE", r"C:\Users\TestUser");

        let path = get_user_gitconfig_path();
        assert!(path.is_some());
        let path_buf = path.unwrap();
        let path_str = path_buf.to_string_lossy();
        assert!(path_str.contains("TestUser"));
        assert!(path_str.ends_with(".gitconfig"));

        // Test alternative Windows behavior (HOMEDRIVE + HOMEPATH)
        env::remove_var("USERPROFILE");
        env::set_var("HOMEDRIVE", "C:");
        env::set_var("HOMEPATH", r"\Users\TestUser2");

        let path = get_user_gitconfig_path();
        assert!(path.is_some());
        let path_buf = path.unwrap();
        let path_str = path_buf.to_string_lossy();
        assert!(path_str.contains("TestUser2"));
        assert!(path_str.ends_with(".gitconfig"));

        // Restore original environment variables
        env::remove_var("HOME");
        env::remove_var("USERPROFILE");
        env::remove_var("HOMEDRIVE");
        env::remove_var("HOMEPATH");

        if let Some(home) = original_home {
            env::set_var("HOME", home);
        }
        if let Some(userprofile) = original_userprofile {
            env::set_var("USERPROFILE", userprofile);
        }
        if let Some(homedrive) = original_homedrive {
            env::set_var("HOMEDRIVE", homedrive);
        }
        if let Some(homepath) = original_homepath {
            env::set_var("HOMEPATH", homepath);
        }
    }

    #[test]
    fn test_system_gitconfig_path_logic() {
        // Test that system config path detection doesn't panic
        let _path = get_system_gitconfig_path();
    }
}
