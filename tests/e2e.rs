//! End-to-end tests: run the real binary non-interactively (`--yes --json`)
//! against throwaway repositories and check the rewritten objects.

use git2::{ObjectType, Oid, Repository};
use serde_json::Value;
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

const SIG: &str =
    "gpgsig -----BEGIN PGP SIGNATURE-----\n \n iQEzBAABCAAdFiEEfake\n -----END PGP SIGNATURE-----";

struct Repo {
    _dir: TempDir,
    repo: Repository,
}

impl Repo {
    /// Linear history; commit `i` (1-based) is authored 2020-01-0i 10:00 +0530.
    fn linear(n: usize, signed: bool) -> Self {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let mut parent: Option<Oid> = None;
        for i in 1..=n {
            let blob = repo.blob(format!("v{i}").as_bytes()).unwrap();
            let mut tb = repo.treebuilder(None).unwrap();
            tb.insert("f.txt", blob, 0o100644).unwrap();
            let tree = tb.write().unwrap();
            let time = 1_577_853_000 + (i as i64 - 1) * 86_400;
            let mut raw = format!("tree {tree}\n");
            if let Some(p) = parent {
                raw += &format!("parent {p}\n");
            }
            raw += &format!(
                "author Old Author <old@example.com> {time} +0530\ncommitter Old Committer <committer@example.com> {time} +0530\n"
            );
            if signed {
                raw += SIG;
                raw += "\n";
            }
            raw += &format!("\ncommit {i}\n");
            let oid = repo
                .odb()
                .unwrap()
                .write(ObjectType::Commit, raw.as_bytes())
                .unwrap();
            parent = Some(oid);
        }
        repo.reference("refs/heads/main", parent.unwrap(), true, "fixture")
            .unwrap();
        repo.set_head("refs/heads/main").unwrap();
        Self { _dir: dir, repo }
    }

    fn path(&self) -> &str {
        self.repo.workdir().unwrap().to_str().unwrap()
    }

    /// Commit ids oldest first.
    fn ids(&self) -> Vec<Oid> {
        let mut walk = self.repo.revwalk().unwrap();
        walk.push_head().unwrap();
        walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::REVERSE)
            .unwrap();
        walk.map(Result::unwrap).collect()
    }

    fn raw(&self, oid: Oid) -> String {
        let odb = self.repo.odb().unwrap();
        let object = odb.read(oid).unwrap();
        String::from_utf8_lossy(object.data()).into_owned()
    }

    fn fsck_clean(&self) {
        if let Ok(out) = Command::new("git")
            .args(["fsck", "--strict", "--no-dangling"])
            .current_dir(self.path())
            .output()
        {
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
}

fn run(args: &[&str], stdin: &str) -> Output {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_git-editor"))
        .args(args)
        .env("GIT_EDITOR_NO_BROWSER", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

fn json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
        panic!(
            "stdout is not JSON ({e}): {}\nstderr: {}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    })
}

#[test]
fn history_json_lists_commits_newest_first() {
    let r = Repo::linear(3, false);
    let out = run(&["-s", "--json", "-r", r.path()], "");
    assert!(out.status.success());
    let doc = json(&out);
    assert_eq!(doc["total_commits"], 3);
    assert_eq!(doc["branch"], "main");
    assert_eq!(doc["commits"][0]["subject"], "commit 3");
    assert_eq!(doc["commits"][0]["number"], 1);
    assert_eq!(
        doc["commits"][2]["author"]["date"],
        "2020-01-01T10:00:00+05:30"
    );
}

#[test]
fn full_rewrite_spreads_dates_oldest_to_newest() {
    let r = Repo::linear(4, false);
    let base = [
        "-r",
        r.path(),
        "--name",
        "New Author",
        "--email",
        "new@example.com",
        "--begin",
        "2024-01-01 00:00:00",
        "--end",
        "2024-01-03 00:00:00",
        "--json",
    ];

    let preview = json(&run(&[&base[..], &["--simulate"]].concat(), ""));
    assert_eq!(preview["commits_to_change"], 4);
    let newest = r.ids()[3].to_string();
    let change = preview["changes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["oid"] == newest.as_str())
        .unwrap();
    assert_eq!(change["date"]["to"], "2024-01-03T00:00:00Z");

    let out = run(&[&base[..], &["--yes"]].concat(), "");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result = json(&out);
    assert_eq!(result["changed"], true);
    assert_eq!(result["backup_ref"], "refs/git-editor/backup/main");

    let ids = r.ids();
    let times: Vec<i64> = ids
        .iter()
        .map(|id| r.repo.find_commit(*id).unwrap().author().when().seconds())
        .collect();
    assert_eq!(times[0], 1_704_067_200);
    assert_eq!(times[3], 1_704_240_000);
    assert!(times.windows(2).all(|w| w[0] < w[1]));
    for id in ids {
        assert!(r.raw(id).contains("author New Author <new@example.com>"));
    }
    r.fsck_clean();
}

#[test]
fn keep_dates_preserves_every_date_and_offset() {
    let r = Repo::linear(3, false);
    let before: Vec<String> = r.ids().iter().map(|id| r.raw(*id)).collect();
    let out = run(
        &[
            "-r",
            r.path(),
            "--name",
            "N",
            "--email",
            "n@example.com",
            "--keep-dates",
            "--committer",
            "keep",
            "--yes",
            "--json",
        ],
        "",
    );
    assert!(out.status.success());
    for (old, id) in before.iter().zip(r.ids()) {
        let new = r.raw(id);
        let old_date = old.lines().find(|l| l.starts_with("author ")).unwrap();
        let new_date = new.lines().find(|l| l.starts_with("author ")).unwrap();
        assert_eq!(old_date.rsplit('>').next(), new_date.rsplit('>').next());
        assert!(new.contains("committer Old Committer <committer@example.com>"));
    }
}

#[test]
fn range_select_keeps_older_commits_byte_identical() {
    let r = Repo::linear(5, true);
    let before = r.ids();
    // Newest-first numbering: 2-3 are the 4th and 3rd commits.
    let out = run(
        &[
            "-x",
            "--select",
            "2-3",
            "--name",
            "Range",
            "-r",
            r.path(),
            "--yes",
            "--json",
        ],
        "",
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let after = r.ids();
    assert_eq!(after[0], before[0]);
    assert_eq!(after[1], before[1]);
    assert!(r.raw(after[1]).contains("gpgsig"));
    assert!(r.raw(after[2]).contains("author Range <old@example.com>"));
    assert!(r.raw(after[4]).contains("author Old Author"));
    assert_eq!(json(&out)["reused"], 2);
    r.fsck_clean();
}

#[test]
fn pick_commit_from_flags() {
    let r = Repo::linear(3, false);
    let target = r.ids()[1].to_string();
    let out = run(
        &[
            "-p",
            "--commit",
            &target[..7],
            "--set-message",
            "reworded",
            "--set-date",
            "2021-02-03 04:05:06 +02:00",
            "-r",
            r.path(),
            "--yes",
            "--json",
        ],
        "",
    );
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let ids = r.ids();
    let raw = r.raw(ids[1]);
    assert!(raw.contains("author Old Author <old@example.com> 1612317906 +0200"));
    assert!(raw.ends_with("\n\nreworded\n"));
}

#[test]
fn detached_head_is_refused() {
    let r = Repo::linear(2, false);
    let head = r.ids()[1];
    r.repo.set_head_detached(head).unwrap();
    let out = run(
        &[
            "-r",
            r.path(),
            "--name",
            "N",
            "--email",
            "n@example.com",
            "--keep-dates",
            "--yes",
            "--json",
        ],
        "",
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(json(&out)["error"].as_str().unwrap().contains("detached"));
    assert!(r.repo.find_reference("refs/heads/HEAD").is_err());
}

#[test]
fn closed_stdin_without_yes_fails_and_writes_nothing() {
    let r = Repo::linear(2, false);
    let before = r.ids();
    let out = run(
        &[
            "-r",
            r.path(),
            "--name",
            "N",
            "--email",
            "n@example.com",
            "--keep-dates",
        ],
        "",
    );
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(r.ids(), before);
}

#[test]
fn declining_exits_130_and_writes_nothing() {
    let r = Repo::linear(2, false);
    let before = r.ids();
    let out = run(
        &[
            "-r",
            r.path(),
            "--name",
            "N",
            "--email",
            "n@example.com",
            "--keep-dates",
            "--json",
        ],
        "no\n",
    );
    assert_eq!(out.status.code(), Some(130));
    assert_eq!(json(&out)["cancelled"], true);
    assert_eq!(r.ids(), before);
}

#[test]
fn url_rewrite_without_clone_dir_is_refused_before_cloning() {
    let out = run(
        &[
            "-r",
            "https://user:SECRET@127.0.0.1:9/o/r.git",
            "--name",
            "N",
            "--email",
            "n@example.com",
            "--keep-dates",
            "--yes",
            "--json",
        ],
        "",
    );
    assert_eq!(out.status.code(), Some(1));
    let error = json(&out)["error"].as_str().unwrap().to_string();
    assert!(error.contains("--clone-dir"), "{error}");
    assert!(!error.contains("SECRET"), "{error}");
}

#[test]
fn header_injection_in_name_is_rejected() {
    let r = Repo::linear(1, false);
    let out = run(
        &[
            "-r",
            r.path(),
            "--name",
            "N\nInjected: x",
            "--email",
            "n@example.com",
            "--keep-dates",
            "--yes",
        ],
        "",
    );
    assert_eq!(out.status.code(), Some(1));
    r.fsck_clean();
}

#[test]
fn conflicting_modes_exit_with_usage_error() {
    let out = run(&["-s", "-x"], "");
    assert_eq!(out.status.code(), Some(2));
}
