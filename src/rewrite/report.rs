//! Human-readable reporting of rewrite results.

use crate::rewrite::engine::Outcome;
use colored::Colorize;

fn short(oid: git2::Oid) -> String {
    oid.to_string()[..8].to_string()
}

pub fn print_outcome(outcome: &Outcome) {
    if crate::output::json_mode() {
        crate::output::emit(&crate::output::outcome_json(outcome));
        return;
    }
    if !outcome.changed() {
        crate::say!(
            "{}",
            "No commits needed rewriting; branch unchanged.".yellow()
        );
        return;
    }
    crate::say!(
        "{} '{}': {} -> {}",
        "Rewritten branch".green(),
        outcome.branch.cyan(),
        short(outcome.old_head).red(),
        short(outcome.new_head).green()
    );
    crate::say!(
        "  {} commit(s) recreated, {} reused unchanged",
        outcome.rewritten.len(),
        outcome.reused
    );
    if outcome.signatures_dropped > 0 {
        crate::say!(
            "  {} {} recreated commit(s) were signed; their signatures were removed (re-sign if needed)",
            "Note:".yellow(),
            outcome.signatures_dropped
        );
    }
    if let Some(backup) = &outcome.backup_ref {
        crate::say!(
            "  Previous tip saved as {} (undo: git reset --keep {})",
            backup.cyan(),
            backup
        );
    }
    if !outcome.stale_refs.is_empty() {
        crate::say!(
            "  {} these refs still point to the old history: {}",
            "Warning:".yellow().bold(),
            outcome.stale_refs.join(", ")
        );
    }
}
