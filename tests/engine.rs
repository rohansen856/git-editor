//! Tests for the history rewrite engine (`git_editor::rewrite::engine`).
//!
//! Fixtures are written as raw commit objects so they can carry signatures,
//! encodings, CRLF messages and non-UTC offsets without needing gpg.

use git2::{ObjectType, Oid, Repository};
use git_editor::rewrite::engine::{self, CommitterMode, Edit, GitTime, Plan};
use std::process::Command;
use tempfile::TempDir;

const AUTHOR: &str = "Old Author <old@example.com> 1577853000 +0530";
const COMMITTER: &str = "Old Committer <committer@example.com> 1577856600 +0530";
const SIG: &str =
    "gpgsig -----BEGIN PGP SIGNATURE-----\n \n iQEzBAABCAAdFiEEfake\n -----END PGP SIGNATURE-----";

struct Fixture {
    _dir: TempDir,
    repo: Repository,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        Self { _dir: dir, repo }
    }

    fn path(&self) -> &std::path::Path {
        self.repo.workdir().unwrap()
    }

    fn tree(&self, content: &str) -> Oid {
        let blob = self.repo.blob(content.as_bytes()).unwrap();
        let mut builder = self.repo.treebuilder(None).unwrap();
        builder.insert("file.txt", blob, 0o100644).unwrap();
        builder.write().unwrap()
    }

    /// Write a raw commit with optional extra header lines.
    fn commit(&self, content: &str, parents: &[Oid], extra: &[&str], message: &[u8]) -> Oid {
        let mut raw = format!("tree {}\n", self.tree(content)).into_bytes();
        for parent in parents {
            raw.extend(format!("parent {parent}\n").as_bytes());
        }
        raw.extend(format!("author {AUTHOR}\ncommitter {COMMITTER}\n").as_bytes());
        for line in extra {
            raw.extend(line.as_bytes());
            raw.push(b'\n');
        }
        raw.push(b'\n');
        raw.extend_from_slice(message);
        self.repo
            .odb()
            .unwrap()
            .write(ObjectType::Commit, &raw)
            .unwrap()
    }

    fn checkout(&self, tip: Oid) {
        self.repo
            .reference("refs/heads/main", tip, true, "fixture")
            .unwrap();
        self.repo.set_head("refs/heads/main").unwrap();
    }

    /// Linear signed history c1 <- c2 <- ... <- cN, returned oldest first.
    fn signed_chain(&self, n: usize) -> Vec<Oid> {
        let mut oids = Vec::new();
        for i in 1..=n {
            let parents: Vec<Oid> = oids.last().copied().into_iter().collect();
            oids.push(self.commit(
                &format!("v{i}"),
                &parents,
                &[SIG],
                format!("commit {i}\n").as_bytes(),
            ));
        }
        self.checkout(*oids.last().unwrap());
        oids
    }

    fn head(&self) -> Oid {
        self.repo.head().unwrap().target().unwrap()
    }

    fn history(&self) -> Vec<Oid> {
        engine::history(&self.repo, self.head()).unwrap()
    }

    fn raw(&self, oid: Oid) -> Vec<u8> {
        let odb = self.repo.odb().unwrap();
        let obj = odb.read(oid).unwrap();
        obj.data().to_vec()
    }

    fn assert_fsck_clean(&self) {
        if let Ok(out) = Command::new("git")
            .args(["fsck", "--strict", "--no-dangling"])
            .current_dir(self.path())
            .output()
        {
            assert!(
                out.status.success(),
                "git fsck failed: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
}

fn plan(edits: Vec<(Oid, Edit)>) -> Plan {
    Plan {
        edits: edits.into_iter().collect(),
        committer: CommitterMode::MatchAuthor,
    }
}

fn rename() -> Edit {
    Edit {
        name: Some("New Author".into()),
        email: Some("new@example.com".into()),
        ..Default::default()
    }
}

fn contains(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|w| w == needle.as_bytes())
}

#[test]
fn untouched_ancestors_keep_ids_and_signatures() {
    let fx = Fixture::new();
    let orig = fx.signed_chain(4);

    let outcome = engine::apply(&fx.repo, &plan(vec![(orig[2], rename())]), fx.head()).unwrap();
    let new = fx.history();

    assert_eq!(new[0], orig[0], "oldest commit must keep its id");
    assert_eq!(new[1], orig[1], "commit before the edit must keep its id");
    assert!(contains(&fx.raw(new[1]), "gpgsig"), "signature kept");
    assert_ne!(new[2], orig[2]);
    assert_ne!(new[3], orig[3], "descendant recreated with new parent");
    assert_eq!(outcome.reused, 2);
    assert_eq!(outcome.rewritten.len(), 2);
    assert_eq!(outcome.signatures_dropped, 2);
    assert!(outcome.changed());
    fx.assert_fsck_clean();
}

#[test]
fn no_edits_is_a_noop() {
    let fx = Fixture::new();
    let orig = fx.signed_chain(3);
    let outcome = engine::apply(&fx.repo, &plan(vec![]), fx.head()).unwrap();
    assert!(!outcome.changed());
    assert_eq!(fx.history(), orig);
    assert_eq!(outcome.reused, 3);
}

#[test]
fn recreated_commits_preserve_encoding_message_bytes_and_offsets() {
    let fx = Fixture::new();
    let latin1 = fx.commit(
        "a",
        &[],
        &["encoding ISO-8859-1"],
        b"caf\xe9 latin1 message\n",
    );
    let crlf = fx.commit("b", &[latin1], &[], b"crlf subject\r\n\r\nbody\r\n");
    fx.checkout(crlf);

    engine::apply(&fx.repo, &plan(vec![(latin1, rename())]), fx.head()).unwrap();
    let new = fx.history();

    let first = fx.raw(new[0]);
    assert!(contains(&first, "encoding ISO-8859-1\n"));
    assert!(first.ends_with(b"\n\ncaf\xe9 latin1 message\n"));
    assert!(contains(
        &first,
        "author New Author <new@example.com> 1577853000 +0530\n"
    ));
    let second = fx.raw(new[1]);
    assert!(second.ends_with(b"\n\ncrlf subject\r\n\r\nbody\r\n"));
    assert!(contains(&second, &format!("author {AUTHOR}\n")));
    fx.assert_fsck_clean();
}

#[test]
fn committer_follows_edited_fields_or_is_kept() {
    let fx = Fixture::new();
    let orig = fx.signed_chain(1);
    let edit = Edit {
        name: Some("New Author".into()),
        time: Some(GitTime::new(1_700_000_000, -300)),
        ..Default::default()
    };

    let mut p = plan(vec![(orig[0], edit.clone())]);
    engine::apply(&fx.repo, &p, fx.head()).unwrap();
    let raw = fx.raw(fx.head());
    assert!(contains(
        &raw,
        "author New Author <old@example.com> 1700000000 -0500\n"
    ));
    assert!(contains(
        &raw,
        "committer New Author <committer@example.com> 1700000000 -0500\n"
    ));

    let fx = Fixture::new();
    let orig = fx.signed_chain(1);
    p = plan(vec![(orig[0], edit)]);
    p.committer = CommitterMode::Keep;
    engine::apply(&fx.repo, &p, fx.head()).unwrap();
    assert!(contains(
        &fx.raw(fx.head()),
        &format!("committer {COMMITTER}\n")
    ));
}

#[test]
fn message_only_edit_keeps_author_and_committer() {
    let fx = Fixture::new();
    let orig = fx.signed_chain(1);
    let edit = Edit {
        message: Some("reworded".into()),
        ..Default::default()
    };
    engine::apply(&fx.repo, &plan(vec![(orig[0], edit)]), fx.head()).unwrap();
    let raw = fx.raw(fx.head());
    assert!(contains(&raw, &format!("author {AUTHOR}\n")));
    assert!(contains(&raw, &format!("committer {COMMITTER}\n")));
    assert!(raw.ends_with(b"\n\nreworded\n"));
}

#[test]
fn merge_parents_are_remapped() {
    let fx = Fixture::new();
    let base = fx.commit("base", &[], &[], b"base\n");
    let left = fx.commit("left", &[base], &[], b"left\n");
    let right = fx.commit("right", &[base], &[], b"right\n");
    let merge = fx.commit("merge", &[left, right], &[], b"merge\n");
    fx.checkout(merge);

    engine::apply(&fx.repo, &plan(vec![(base, rename())]), fx.head()).unwrap();
    let head = fx.repo.find_commit(fx.head()).unwrap();
    assert_eq!(head.parent_count(), 2);
    let new_base = fx.history()[0];
    for parent in head.parents() {
        assert_eq!(parent.parent_id(0).unwrap(), new_base);
    }
    fx.assert_fsck_clean();
}

#[test]
fn moved_branch_is_rejected_without_changes() {
    let fx = Fixture::new();
    let orig = fx.signed_chain(2);
    let err = engine::apply(&fx.repo, &plan(vec![(orig[0], rename())]), orig[0]).unwrap_err();
    assert!(err.to_string().contains("moved"));
    assert_eq!(fx.head(), orig[1]);
}

#[test]
fn identity_with_header_characters_is_rejected() {
    let fx = Fixture::new();
    let orig = fx.signed_chain(1);
    for bad in ["New\nInjected: header", "a <b>", ""] {
        let edit = Edit {
            name: Some(bad.into()),
            ..Default::default()
        };
        assert!(engine::apply(&fx.repo, &plan(vec![(orig[0], edit)]), fx.head()).is_err());
    }
    assert_eq!(fx.head(), orig[0]);
}

#[test]
fn trailers_follow_identity_change() {
    let fx = Fixture::new();
    let c = fx.commit(
        "a",
        &[],
        &[],
        b"fix\n\nSigned-off-by: Old Author <old@example.com>\n",
    );
    fx.checkout(c);
    engine::apply(&fx.repo, &plan(vec![(c, rename())]), fx.head()).unwrap();
    assert!(fx
        .raw(fx.head())
        .ends_with(b"\n\nfix\n\nSigned-off-by: New Author <new@example.com>\n"));
}

#[test]
fn detached_head_is_rejected() {
    let fx = Fixture::new();
    let orig = fx.signed_chain(2);
    fx.repo.set_head_detached(orig[1]).unwrap();
    let err = engine::current_branch(&fx.repo).unwrap_err();
    assert!(err.to_string().contains("detached"));
    let err = engine::apply(&fx.repo, &plan(vec![(orig[0], rename())]), orig[1]).unwrap_err();
    assert!(err.to_string().contains("detached"));
    assert!(fx.repo.find_reference("refs/heads/HEAD").is_err());
}

#[test]
fn unborn_branch_is_rejected() {
    let fx = Fixture::new();
    let err = engine::current_branch(&fx.repo).unwrap_err();
    assert!(err.to_string().contains("no commits"));
}

#[test]
fn backup_ref_and_stale_refs_are_reported() {
    let fx = Fixture::new();
    let orig = fx.signed_chain(3);
    fx.repo
        .branch("side", &fx.repo.find_commit(orig[1]).unwrap(), false)
        .unwrap();
    fx.repo
        .tag_lightweight(
            "v1",
            fx.repo.find_commit(orig[2]).unwrap().as_object(),
            false,
        )
        .unwrap();
    fx.repo
        .branch("unrelated", &fx.repo.find_commit(orig[0]).unwrap(), false)
        .unwrap();

    let outcome = engine::apply(&fx.repo, &plan(vec![(orig[1], rename())]), fx.head()).unwrap();

    assert_eq!(
        outcome.backup_ref.as_deref(),
        Some("refs/git-editor/backup/main")
    );
    let backup = fx
        .repo
        .find_reference("refs/git-editor/backup/main")
        .unwrap();
    assert_eq!(backup.target(), Some(orig[2]));
    assert_eq!(outcome.stale_refs, vec!["refs/heads/side", "refs/tags/v1"]);
}
