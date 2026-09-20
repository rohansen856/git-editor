//! Output routing.
//!
//! Human-readable progress goes through [`say!`](crate::say) /
//! [`say_inline!`](crate::say_inline): stdout normally, stderr in `--json`
//! mode, so stdout carries nothing but the JSON document.

use std::sync::atomic::{AtomicBool, Ordering};

static JSON_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_json_mode(enabled: bool) {
    JSON_MODE.store(enabled, Ordering::Relaxed);
}

pub fn json_mode() -> bool {
    JSON_MODE.load(Ordering::Relaxed)
}

/// `println!` that moves to stderr in `--json` mode.
#[macro_export]
macro_rules! say {
    ($($arg:tt)*) => {
        if $crate::output::json_mode() {
            eprintln!($($arg)*)
        } else {
            println!($($arg)*)
        }
    };
}

/// `print!` that moves to stderr in `--json` mode.
#[macro_export]
macro_rules! say_inline {
    ($($arg:tt)*) => {
        if $crate::output::json_mode() {
            eprint!($($arg)*)
        } else {
            print!($($arg)*)
        }
    };
}

use crate::rewrite::engine::{GitTime, Outcome};
use crate::utils::dates::to_rfc3339;
use crate::utils::simulation::SimulationResult;
use crate::utils::types::CommitInfo;
use serde_json::{json, Value};

/// Print one JSON document on stdout.
pub fn emit(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    );
}

fn person(name: &str, email: &str, seconds: i64, offset: i32) -> Value {
    json!({
        "name": name,
        "email": email,
        "date": to_rfc3339(GitTime::new(seconds, offset)),
    })
}

/// `-s --json`: the branch history, newest first (numbers match `--commit`/`--select`).
pub fn history_json(branch: Option<&str>, head: Option<String>, commits: &[CommitInfo]) -> Value {
    let commits: Vec<Value> = commits
        .iter()
        .enumerate()
        .map(|(i, c)| {
            json!({
                "number": i + 1,
                "oid": c.oid.to_string(),
                "short": c.short_hash,
                "author": person(&c.author_name, &c.author_email, c.timestamp.and_utc().timestamp(), c.author_offset_min),
                "committer": person(&c.committer_name, &c.committer_email, c.committer_timestamp.and_utc().timestamp(), c.committer_offset_min),
                "subject": c.message.lines().next().unwrap_or(""),
                "message": c.message,
                "message_is_utf8": c.message_is_utf8,
                "parents": c.parent_count,
            })
        })
        .collect();
    json!({
        "ok": true,
        "command": "history",
        "branch": branch,
        "head": head,
        "total_commits": commits.len(),
        "commits": commits,
    })
}

fn from_to<T: ToString>(from: T, to: &Option<T>) -> Value {
    to.as_ref().map_or(
        Value::Null,
        |to| json!({ "from": from.to_string(), "to": to.to_string() }),
    )
}

/// Preview of a plan (`--simulate`, or the summary before a confirmation).
pub fn simulation_json(result: &SimulationResult) -> Value {
    let utc = |t: chrono::NaiveDateTime| t.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let changes: Vec<Value> = result
        .changes
        .iter()
        .filter(|c| c.has_changes())
        .map(|c| {
            json!({
                "oid": c.commit_oid.to_string(),
                "short": c.short_hash,
                "author_name": from_to(c.original_author.clone(), &c.new_author),
                "author_email": from_to(c.original_email.clone(), &c.new_email),
                "date": from_to(utc(c.original_timestamp), &c.new_timestamp.map(utc)),
                "message": from_to(c.original_message.clone(), &c.new_message),
            })
        })
        .collect();
    json!({
        "ok": true,
        "command": "simulate",
        "mode": result.operation_mode,
        "total_commits": result.stats.total_commits,
        "commits_to_change": result.stats.commits_to_change,
        "changes": changes,
    })
}

/// Result of a rewrite.
pub fn outcome_json(outcome: &Outcome) -> Value {
    let rewritten: Vec<Value> = outcome
        .rewritten
        .iter()
        .map(|(old, new)| json!({ "old": old.to_string(), "new": new.to_string() }))
        .collect();
    json!({
        "ok": true,
        "command": "rewrite",
        "changed": outcome.changed(),
        "branch": outcome.branch,
        "old_head": outcome.old_head.to_string(),
        "new_head": outcome.new_head.to_string(),
        "backup_ref": outcome.backup_ref,
        "rewritten": rewritten,
        "reused": outcome.reused,
        "signatures_dropped": outcome.signatures_dropped,
        "stale_refs": outcome.stale_refs,
    })
}

/// Error document; `exit_code` mirrors the process status (1 error, 130 cancelled).
pub fn error_json(message: &str, exit_code: i32) -> Value {
    json!({
        "ok": false,
        "error": message,
        "cancelled": exit_code == 130,
        "exit_code": exit_code,
    })
}
