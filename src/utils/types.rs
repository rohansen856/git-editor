use chrono::NaiveDateTime;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: git2::Oid,
    pub short_hash: String,
    pub timestamp: NaiveDateTime,
    pub author_name: String,
    pub author_email: String,
    pub message: String,
    pub parent_count: usize,
}
#[derive(Default)]
pub struct EditOptions {
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub timestamp: Option<NaiveDateTime>,
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_commit_info_creation() {
        let oid = git2::Oid::from_str("1234567890abcdef1234567890abcdef12345678").unwrap();
        let timestamp = chrono::DateTime::from_timestamp(1234567890, 0)
            .unwrap()
            .naive_utc();

        let commit_info = CommitInfo {
            oid,
            short_hash: "12345678".to_string(),
            timestamp,
            author_name: "Test User".to_string(),
            author_email: "test@example.com".to_string(),
            message: "Test commit message".to_string(),
            parent_count: 1,
        };

        assert_eq!(commit_info.oid, oid);
        assert_eq!(commit_info.short_hash, "12345678");
        assert_eq!(commit_info.timestamp, timestamp);
        assert_eq!(commit_info.author_name, "Test User");
        assert_eq!(commit_info.author_email, "test@example.com");
        assert_eq!(commit_info.message, "Test commit message");
        assert_eq!(commit_info.parent_count, 1);
    }

    #[test]
    fn test_edit_options_default() {
        let options = EditOptions::default();

        assert_eq!(options.author_name, None);
        assert_eq!(options.author_email, None);
        assert_eq!(options.timestamp, None);
        assert_eq!(options.message, None);
    }

    #[test]
    fn test_edit_options_with_values() {
        let timestamp = chrono::DateTime::from_timestamp(1234567890, 0)
            .unwrap()
            .naive_utc();

        let options = EditOptions {
            author_name: Some("New Author".to_string()),
            author_email: Some("new@example.com".to_string()),
            timestamp: Some(timestamp),
            message: Some("New commit message".to_string()),
        };

        assert_eq!(options.author_name, Some("New Author".to_string()));
        assert_eq!(options.author_email, Some("new@example.com".to_string()));
        assert_eq!(options.timestamp, Some(timestamp));
        assert_eq!(options.message, Some("New commit message".to_string()));
    }

    #[test]
    fn test_edit_options_partial_values() {
        let options = EditOptions {
            author_name: Some("New Author".to_string()),
            author_email: None,
            timestamp: None,
            message: Some("New message".to_string()),
        };

        assert_eq!(options.author_name, Some("New Author".to_string()));
        assert_eq!(options.author_email, None);
        assert_eq!(options.timestamp, None);
        assert_eq!(options.message, Some("New message".to_string()));
    }

    #[test]
    fn test_result_type_alias() {
        fn test_function() -> Result<String> {
            Ok("test".to_string())
        }

        let result = test_function();
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test");
    }

    #[test]
    fn test_result_type_alias_error() {
        fn test_function() -> Result<String> {
            Err("error".into())
        }

        let result = test_function();
        assert!(result.is_err());
    }
}
