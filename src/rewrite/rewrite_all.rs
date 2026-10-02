use crate::args::Args;
use crate::rewrite::engine::{self, CommitterMode, Edit, GitTime, Outcome, Plan};
use crate::rewrite::report::print_outcome;
use crate::utils::dates::parse_git_time;
use crate::utils::types::Result;
use chrono::NaiveDateTime;
use git2::{Oid, Repository};

/// Plan that gives every commit reachable from `tip` the new identity.
///
/// `timestamps` (UTC instants, oldest commit first) replaces each author date
/// and is written with `offset_minutes`; `None` keeps every commit's own date
/// and time-zone offset.
pub fn full_rewrite_plan(
    repo: &Repository,
    tip: Oid,
    name: &str,
    email: &str,
    timestamps: Option<&[NaiveDateTime]>,
    offset_minutes: i32,
    committer: CommitterMode,
) -> Result<Plan> {
    let history = engine::history(repo, tip)?;
    if let Some(ts) = timestamps {
        if ts.len() != history.len() {
            return Err(format!(
                "Planned {} timestamps for {} commits; the history changed, re-run the command",
                ts.len(),
                history.len()
            )
            .into());
        }
    }
    let edits = history
        .iter()
        .enumerate()
        .map(|(i, oid)| {
            let edit = Edit {
                name: Some(name.to_string()),
                email: Some(email.to_string()),
                time: timestamps
                    .map(|ts| GitTime::new(ts[i].and_utc().timestamp(), offset_minutes)),
                message: None,
            };
            (*oid, edit)
        })
        .collect();
    Ok(Plan { edits, committer })
}

/// Offset (minutes) given with `--begin`, used for generated timestamps; `0` = UTC.
pub fn begin_offset(args: &Args) -> i32 {
    args.start
        .as_deref()
        .and_then(|s| parse_git_time(s).ok())
        .map_or(0, |t| t.offset_minutes)
}

/// Rewrite every commit on the current branch, which must still be at `expected_head`.
pub fn rewrite_all_commits(
    args: &Args,
    timestamps: Option<&[NaiveDateTime]>,
    expected_head: Oid,
) -> Result<Outcome> {
    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;
    let plan = full_rewrite_plan(
        &repo,
        expected_head,
        args.name.as_ref().unwrap(),
        args.email.as_ref().unwrap(),
        timestamps,
        begin_offset(args),
        args.committer.into(),
    )?;
    apply_plan(&repo, &plan, expected_head)
}

/// Apply a previously previewed plan and report the result.
pub fn apply_plan(repo: &Repository, plan: &Plan, expected_head: Oid) -> Result<Outcome> {
    let outcome = engine::apply(repo, plan, expected_head)?;
    print_outcome(&outcome);
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use git2::{Signature, Time};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn rewrite_all_updates_signed_off_by_trailer() {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_str().unwrap().to_string();
        let repo = Repository::init(&repo_path).unwrap();

        let file_path = temp_dir.path().join("file.txt");
        fs::write(&file_path, "content").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = Signature::new(
            "Old Author",
            "old@example.com",
            &Time::new(1_700_000_000, 0),
        )
        .unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Fix bug\n\nSigned-off-by: Old Author <old@example.com>\n",
            &tree,
            &[],
        )
        .unwrap();

        let args = Args {
            repo_path: Some(repo_path.clone()),
            email: Some("new@example.com".to_string()),
            name: Some("New Author".to_string()),
            ..Default::default()
        };

        let ts = NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let head = repo.head().unwrap().target().unwrap();
        rewrite_all_commits(&args, Some(&[ts]), head).unwrap();

        let repo = Repository::open(&repo_path).unwrap();
        let tip = repo.head().unwrap().peel_to_commit().unwrap();
        let msg = tip.message().unwrap_or("");
        assert!(
            msg.contains("Signed-off-by: New Author <new@example.com>"),
            "expected rewritten Signed-off-by, got: {msg}"
        );
        assert!(!msg.contains("old@example.com"));
        assert_eq!(tip.author().email().ok(), Some("new@example.com"));
    }
}
