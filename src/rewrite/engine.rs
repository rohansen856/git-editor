//! History rewrite engine shared by every write mode.
//!
//! Callers describe *what* should change with a [`Plan`] (edits keyed by the
//! original commit id); [`apply`] recreates only the commits that must change:
//!
//! * a commit with no edit whose parents are unchanged is reused as-is, so its
//!   id, signature and every header stay byte-identical;
//! * a recreated commit keeps its raw headers (`encoding`, `mergetag`, …) and
//!   raw message bytes; only `parent`, the edited `author`/`committer` fields
//!   and the now-invalid `gpgsig` headers change;
//! * the branch ref is moved with a compare-and-swap against the tip the plan
//!   was built from.

use crate::utils::message_trailers::rewrite_author_trailers;
use crate::utils::types::Result;
use git2::{ErrorCode, ObjectType, Oid, Repository, Sort};
use std::collections::HashMap;

/// What happens to the committer of a commit whose author fields are edited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommitterMode {
    /// Committer name/email/date follow whichever author fields were edited.
    #[default]
    MatchAuthor,
    /// Committer line is left byte-identical.
    Keep,
}

/// A git timestamp: seconds since the epoch plus the UTC offset in minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GitTime {
    pub seconds: i64,
    pub offset_minutes: i32,
}

impl GitTime {
    pub fn new(seconds: i64, offset_minutes: i32) -> Self {
        Self {
            seconds,
            offset_minutes,
        }
    }

    fn header_value(&self) -> String {
        let sign = if self.offset_minutes < 0 { '-' } else { '+' };
        let abs = self.offset_minutes.abs();
        format!("{} {}{:02}{:02}", self.seconds, sign, abs / 60, abs % 60)
    }
}

/// Requested changes for one commit. `None` keeps the original value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Edit {
    pub name: Option<String>,
    pub email: Option<String>,
    pub time: Option<GitTime>,
    pub message: Option<String>,
}

impl Edit {
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.email.is_none() && self.time.is_none() && self.message.is_none()
    }

    fn touches_author(&self) -> bool {
        self.name.is_some() || self.email.is_some() || self.time.is_some()
    }
}

/// Edits keyed by original commit id, plus the committer policy.
#[derive(Debug, Clone, Default)]
pub struct Plan {
    pub edits: HashMap<Oid, Edit>,
    pub committer: CommitterMode,
}

/// The branch a rewrite will move.
#[derive(Debug, Clone)]
pub struct BranchHead {
    pub refname: String,
    pub shorthand: String,
    pub head: Oid,
}

/// Result of [`apply`].
#[derive(Debug, Clone)]
pub struct Outcome {
    pub branch: String,
    pub old_head: Oid,
    pub new_head: Oid,
    /// `(original, rewritten)` for every commit that was recreated.
    pub rewritten: Vec<(Oid, Oid)>,
    /// Commits reused unchanged (same id).
    pub reused: usize,
    /// Recreated commits that carried a signature which had to be dropped.
    pub signatures_dropped: usize,
    /// Ref holding the pre-rewrite tip (`refs/git-editor/backup/<branch>`).
    pub backup_ref: Option<String>,
    /// Other branches/tags that still contain rewritten (old) commits.
    pub stale_refs: Vec<String>,
}

impl Outcome {
    pub fn changed(&self) -> bool {
        self.old_head != self.new_head
    }
}

/// Resolve the checked-out branch, rejecting states a rewrite cannot handle.
pub fn current_branch(repo: &Repository) -> Result<BranchHead> {
    if repo.head_detached().unwrap_or(false) {
        return Err(
            "HEAD is detached; check out a branch before rewriting (e.g. `git switch main`)".into(),
        );
    }
    let head = match repo.head() {
        Ok(head) => head,
        Err(e) if matches!(e.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) => {
            return Err("Repository has no commits yet; nothing to rewrite".into())
        }
        Err(e) => return Err(e.into()),
    };
    if !head.is_branch() {
        return Err("HEAD does not point to a local branch; check out a branch first".into());
    }
    let refname = head
        .name()
        .map_err(|_| "HEAD reference name is not valid UTF-8")?
        .to_string();
    let shorthand = head.shorthand().unwrap_or(&refname).to_string();
    let head_oid = head.target().ok_or("HEAD is a symbolic reference")?;
    Ok(BranchHead {
        refname,
        shorthand,
        head: head_oid,
    })
}

/// Commits reachable from `tip`, oldest first (parents before children).
pub fn history(repo: &Repository, tip: Oid) -> Result<Vec<Oid>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.push(tip)?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    let mut oids = revwalk.collect::<std::result::Result<Vec<_>, _>>()?;
    oids.reverse();
    Ok(oids)
}

/// Reject values that would corrupt a commit header line.
pub fn validate_identity_part(value: &str, what: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(format!("{what} cannot be empty").into());
    }
    if let Some(c) = value
        .chars()
        .find(|c| matches!(c, '<' | '>' | '\n' | '\r' | '\0'))
    {
        return Err(format!("{what} contains an invalid character {c:?}").into());
    }
    Ok(())
}

/// Apply `plan` to the checked-out branch, which must still point at `expected_head`.
pub fn apply(repo: &Repository, plan: &Plan, expected_head: Oid) -> Result<Outcome> {
    for edit in plan.edits.values() {
        if let Some(name) = &edit.name {
            validate_identity_part(name, "Author name")?;
        }
        if let Some(email) = &edit.email {
            validate_identity_part(email, "Author email")?;
        }
    }

    let branch = current_branch(repo)?;
    if branch.head != expected_head {
        return Err(format!(
            "Branch '{}' moved while preparing the rewrite (expected {}, found {}); nothing was changed, re-run the command",
            branch.shorthand, expected_head, branch.head
        )
        .into());
    }

    let odb = repo.odb()?;
    let mut new_ids: HashMap<Oid, Oid> = HashMap::new();
    let mut rewritten = Vec::new();
    let mut reused = 0;
    let mut signatures_dropped = 0;

    for oid in history(repo, branch.head)? {
        let commit = repo.find_commit(oid)?;
        let parents: Vec<Oid> = commit
            .parent_ids()
            .map(|p| *new_ids.get(&p).unwrap_or(&p))
            .collect();
        let parents_changed = parents
            .iter()
            .ne(commit.parent_ids().collect::<Vec<_>>().iter());
        let edit = plan.edits.get(&oid).filter(|e| !e.is_empty());

        if edit.is_none() && !parents_changed {
            new_ids.insert(oid, oid);
            reused += 1;
            continue;
        }

        let (buffer, had_signature) = build_commit(
            &commit,
            &parents,
            edit.unwrap_or(&Edit::default()),
            plan.committer,
        )?;
        let new_oid = odb.write(ObjectType::Commit, &buffer)?;
        if had_signature {
            signatures_dropped += 1;
        }
        if new_oid == oid {
            reused += 1;
        } else {
            rewritten.push((oid, new_oid));
        }
        new_ids.insert(oid, new_oid);
    }

    let new_head = *new_ids.get(&branch.head).unwrap_or(&branch.head);
    let mut backup_ref = None;
    let mut stale_refs = Vec::new();
    if new_head != branch.head {
        let backup = format!("{BACKUP_PREFIX}{}", branch.shorthand);
        repo.reference(
            &backup,
            branch.head,
            true,
            "git-editor: tip before history rewrite",
        )?;
        repo.reference_matching(
            &branch.refname,
            new_head,
            true,
            branch.head,
            "git-editor: history rewritten",
        )?;
        backup_ref = Some(backup);
        stale_refs = refs_containing(repo, &branch.refname, &rewritten, &new_ids)?;
    }

    Ok(Outcome {
        branch: branch.shorthand,
        old_head: branch.head,
        new_head,
        rewritten,
        reused,
        signatures_dropped,
        backup_ref,
        stale_refs,
    })
}

/// Namespace of the backup refs written before every rewrite.
pub const BACKUP_PREFIX: &str = "refs/git-editor/backup/";

/// Local branches and tags (other than `current`) that still reach a rewritten commit.
fn refs_containing(
    repo: &Repository,
    current: &str,
    rewritten: &[(Oid, Oid)],
    new_ids: &HashMap<Oid, Oid>,
) -> Result<Vec<String>> {
    // Only the earliest rewritten commits matter: anything reaching a later
    // rewritten commit also reaches one of these.
    let roots: Vec<Oid> = rewritten
        .iter()
        .map(|(old, _)| *old)
        .filter(|old| {
            repo.find_commit(*old)
                .map(|c| c.parent_ids().all(|p| new_ids.get(&p) == Some(&p)))
                .unwrap_or(false)
        })
        .collect();

    let mut stale = Vec::new();
    for reference in repo.references()? {
        let reference = reference?;
        let Ok(name) = reference.name() else {
            continue;
        };
        if name == current || !(name.starts_with("refs/heads/") || name.starts_with("refs/tags/")) {
            continue;
        }
        let Ok(target) = reference.peel_to_commit() else {
            continue;
        };
        let target = target.id();
        let reaches_old = roots.iter().any(|root| {
            target == *root || repo.graph_descendant_of(target, *root).unwrap_or(false)
        });
        if reaches_old {
            stale.push(name.to_string());
        }
    }
    stale.sort();
    Ok(stale)
}

/// One header field of a raw commit: its key and the full bytes of the field
/// (key, value and continuation lines, each terminated by `\n`).
struct HeaderField<'a> {
    key: &'a [u8],
    raw: &'a [u8],
}

fn parse_header(raw: &[u8]) -> Vec<HeaderField<'_>> {
    let mut fields: Vec<HeaderField<'_>> = Vec::new();
    let mut start = 0;
    while start < raw.len() {
        let end = raw[start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(raw.len(), |p| start + p + 1);
        if raw[start] == b' ' {
            // Continuation line: extend the previous field.
            if let Some(last) = fields.last_mut() {
                let field_start = last.raw.as_ptr() as usize - raw.as_ptr() as usize;
                last.raw = &raw[field_start..end];
            }
        } else {
            let line = &raw[start..end];
            let key_len = line.iter().position(|&b| b == b' ').unwrap_or(line.len());
            fields.push(HeaderField {
                key: &line[..key_len],
                raw: line,
            });
        }
        start = end;
    }
    fields
}

/// Split a raw `Name <email> seconds offset` signature value.
struct RawSignature {
    name: Vec<u8>,
    email: Vec<u8>,
    time: Vec<u8>,
}

fn parse_signature(field: &[u8], key: &[u8]) -> Option<RawSignature> {
    let value = field.strip_prefix(key)?.strip_prefix(b" ")?;
    let value = value.strip_suffix(b"\n").unwrap_or(value);
    let lt = value.iter().position(|&b| b == b'<')?;
    let gt = value.iter().rposition(|&b| b == b'>')?;
    if gt < lt {
        return None;
    }
    let name = value[..lt]
        .strip_suffix(b" ")
        .unwrap_or(&value[..lt])
        .to_vec();
    let email = value[lt + 1..gt].to_vec();
    let time = value[gt + 1..]
        .strip_prefix(b" ")
        .unwrap_or(&value[gt + 1..])
        .to_vec();
    Some(RawSignature { name, email, time })
}

fn signature_line(key: &str, name: &[u8], email: &[u8], time: &[u8]) -> Vec<u8> {
    let mut line = Vec::with_capacity(key.len() + name.len() + email.len() + time.len() + 8);
    line.extend_from_slice(key.as_bytes());
    line.push(b' ');
    line.extend_from_slice(name);
    line.extend_from_slice(b" <");
    line.extend_from_slice(email);
    line.extend_from_slice(b"> ");
    line.extend_from_slice(time);
    line.push(b'\n');
    line
}

/// Build the raw bytes of the recreated commit. Returns whether a signature was dropped.
fn build_commit(
    commit: &git2::Commit<'_>,
    parents: &[Oid],
    edit: &Edit,
    committer_mode: CommitterMode,
) -> Result<(Vec<u8>, bool)> {
    let header = commit.raw_header_bytes();
    let fields = parse_header(header);
    let mut out = Vec::with_capacity(header.len() + commit.message_raw_bytes().len() + 64);
    let mut had_signature = false;

    let orig_author = fields
        .iter()
        .find(|f| f.key == b"author")
        .and_then(|f| parse_signature(f.raw, b"author"))
        .ok_or("commit has an unparsable author line")?;
    let new_name = edit
        .name
        .as_ref()
        .map_or(orig_author.name.clone(), |n| n.as_bytes().to_vec());
    let new_email = edit
        .email
        .as_ref()
        .map_or(orig_author.email.clone(), |e| e.as_bytes().to_vec());
    let new_time = edit
        .time
        .map_or(orig_author.time.clone(), |t| t.header_value().into_bytes());

    for field in &fields {
        match field.key {
            b"tree" => {
                out.extend_from_slice(field.raw);
                for parent in parents {
                    out.extend_from_slice(format!("parent {parent}\n").as_bytes());
                }
            }
            b"parent" => {}
            b"author" if edit.touches_author() => {
                out.extend(signature_line("author", &new_name, &new_email, &new_time));
            }
            b"committer"
                if edit.touches_author() && committer_mode == CommitterMode::MatchAuthor =>
            {
                let orig = parse_signature(field.raw, b"committer")
                    .ok_or("commit has an unparsable committer line")?;
                let name = if edit.name.is_some() {
                    &new_name
                } else {
                    &orig.name
                };
                let email = if edit.email.is_some() {
                    &new_email
                } else {
                    &orig.email
                };
                let time = if edit.time.is_some() {
                    &new_time
                } else {
                    &orig.time
                };
                out.extend(signature_line("committer", name, email, time));
            }
            b"gpgsig" | b"gpgsig-sha256" => had_signature = true,
            _ => out.extend_from_slice(field.raw),
        }
    }

    out.push(b'\n');
    out.extend(new_message(
        commit,
        edit,
        &orig_author,
        &new_name,
        &new_email,
    ));
    Ok((out, had_signature))
}

fn new_message(
    commit: &git2::Commit<'_>,
    edit: &Edit,
    orig_author: &RawSignature,
    new_name: &[u8],
    new_email: &[u8],
) -> Vec<u8> {
    let base: Vec<u8> = match &edit.message {
        Some(message) if message.ends_with('\n') => message.as_bytes().to_vec(),
        Some(message) => format!("{message}\n").into_bytes(),
        None => commit.message_raw_bytes().to_vec(),
    };
    let identity_changed = edit.name.is_some() || edit.email.is_some();
    if !identity_changed {
        return base;
    }
    match (
        std::str::from_utf8(&base),
        std::str::from_utf8(&orig_author.name),
        std::str::from_utf8(&orig_author.email),
        std::str::from_utf8(new_name),
        std::str::from_utf8(new_email),
    ) {
        (Ok(message), Ok(old_name), Ok(old_email), Ok(name), Ok(email)) => {
            rewrite_author_trailers(message, old_name, old_email, name, email).into_bytes()
        }
        // Non-UTF-8 message or identity: keep the bytes untouched.
        _ => base,
    }
}
