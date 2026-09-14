//! Human-readable reporting of rewrite results.

use crate::rewrite::engine::Outcome;
use colored::Colorize;

fn short(oid: git2::Oid) -> String {
    oid.to_string()[..8].to_string()
}

pub fn print_outcome(outcome: &Outcome) {
    if !outcome.changed() {
        println!(
            "{}",
            "No commits needed rewriting; branch unchanged.".yellow()
        );
        return;
    }
    println!(
        "{} '{}': {} -> {}",
        "Rewritten branch".green(),
        outcome.branch.cyan(),
        short(outcome.old_head).red(),
        short(outcome.new_head).green()
    );
    println!(
        "  {} commit(s) recreated, {} reused unchanged",
        outcome.rewritten.len(),
        outcome.reused
    );
    if outcome.signatures_dropped > 0 {
        println!(
            "  {} {} recreated commit(s) were signed; their signatures were removed (re-sign if needed)",
            "Note:".yellow(),
            outcome.signatures_dropped
        );
    }
    if let Some(backup) = &outcome.backup_ref {
        println!(
            "  Previous tip saved as {} (undo: git reset --keep {})",
            backup.cyan(),
            backup
        );
    }
    if !outcome.stale_refs.is_empty() {
        println!(
            "  {} these refs still point to the old history: {}",
            "Warning:".yellow().bold(),
            outcome.stale_refs.join(", ")
        );
    }
}
