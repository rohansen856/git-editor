use chrono::NaiveDateTime;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

/// Snapshot of one commit, as listed by `get_commit_history`.
///
/// `timestamp` is the **author** date in UTC; `author_offset_min` keeps the
/// author's original time-zone offset. Committer fields are tracked separately
/// so rewrites can preserve them. Name, email and message are lossy UTF-8
/// renderings meant for display; the rewrite engine always reads the raw
/// commit object, never these strings.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: git2::Oid,
    pub short_hash: String,
    pub timestamp: NaiveDateTime,
    pub author_offset_min: i32,
    pub author_name: String,
    pub author_email: String,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_timestamp: NaiveDateTime,
    pub committer_offset_min: i32,
    pub message: String,
    pub message_is_utf8: bool,
    pub parent_count: usize,
}

impl Default for CommitInfo {
    fn default() -> Self {
        Self {
            oid: git2::Oid::ZERO_SHA1,
            short_hash: String::new(),
            timestamp: NaiveDateTime::default(),
            author_offset_min: 0,
            author_name: String::new(),
            author_email: String::new(),
            committer_name: String::new(),
            committer_email: String::new(),
            committer_timestamp: NaiveDateTime::default(),
            committer_offset_min: 0,
            message: String::new(),
            message_is_utf8: true,
            parent_count: 0,
        }
    }
}

#[derive(Default)]
pub struct EditOptions {
    pub author_name: Option<String>,
    pub author_email: Option<String>,
    pub timestamp: Option<crate::rewrite::engine::GitTime>,
    pub message: Option<String>,
}
