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
