use crate::utils::message_trailers::rewrite_author_trailers;
use crate::utils::types::Result;
use crate::{args::Args, utils::commit_history::get_commit_history};
use chrono::NaiveDateTime;
use colored::Colorize;
use git2::{Repository, Signature, Sort, Time};
use std::collections::HashMap;

pub fn rewrite_all_commits(args: &Args, timestamps: Vec<NaiveDateTime>) -> Result<()> {
    let repo = Repository::open(args.repo_path.as_ref().unwrap())?;
    let head_ref = repo.head()?;
    let branch_name = head_ref
        .shorthand()
        .ok_or("Detached HEAD or invalid branch")?;
    let full_ref = format!("refs/heads/{branch_name}");

    let new_name = args.name.as_ref().unwrap();
    let new_email = args.email.as_ref().unwrap();

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;
    let mut orig_oids: Vec<_> = revwalk.collect::<std::result::Result<Vec<_>, _>>()?;
    orig_oids.reverse();

    let mut new_map: HashMap<git2::Oid, git2::Oid> = HashMap::new();
    let mut last_new_oid = None;

    for (i, &oid) in orig_oids.iter().enumerate() {
        let orig = repo.find_commit(oid)?;
        let tree = orig.tree()?;

        let new_parents: Result<Vec<_>> = orig
            .parent_ids()
            .map(|pid| {
                let new_pid = *new_map.get(&pid).unwrap_or(&pid);
                repo.find_commit(new_pid).map_err(|e| e.into())
            })
            .collect();

        let timestamp: i64 = timestamps[i].and_utc().timestamp();
        let sig = Signature::new(new_name, new_email, &Time::new(timestamp, 0))?;

        let old_author = orig.author();
        let message = rewrite_author_trailers(
            orig.message().unwrap_or_default(),
            old_author.name().unwrap_or(""),
            old_author.email().unwrap_or(""),
            new_name,
            new_email,
        );

        let new_oid = repo.commit(
            None,
            &sig,
            &sig,
            &message,
            &tree,
            &new_parents?.iter().collect::<Vec<_>>(),
        )?;

        new_map.insert(oid, new_oid);
        last_new_oid = Some(new_oid);
    }

    if let Some(new_head) = last_new_oid {
        repo.reference(&full_ref, new_head, true, "history rewritten")?;
        println!(
            "{} '{}' -> {}",
            "Rewritten branch".green(),
            branch_name.cyan(),
            new_head.to_string().cyan()
        );
        if args.show_history {
            get_commit_history(args, true)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
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
            start: None,
            end: None,
            show_history: false,
            pick_specific_commits: false,
            range: false,
            simulate: false,
            show_diff: false,
            edit_message: false,
            edit_author: false,
            edit_time: false,
            skip_range_check: false,
            docs: false,
            _temp_dir: None,
        };

        let ts = NaiveDate::from_ymd_opt(2024, 1, 1)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        rewrite_all_commits(&args, vec![ts]).unwrap();

        let repo = Repository::open(&repo_path).unwrap();
        let tip = repo.head().unwrap().peel_to_commit().unwrap();
        let msg = tip.message().unwrap_or("");
        assert!(
            msg.contains("Signed-off-by: New Author <new@example.com>"),
            "expected rewritten Signed-off-by, got: {msg}"
        );
        assert!(!msg.contains("old@example.com"));
        assert_eq!(tip.author().email(), Some("new@example.com"));
    }
}
