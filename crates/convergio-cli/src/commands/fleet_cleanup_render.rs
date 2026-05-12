//! Rendering for `cvg fleet cleanup`. Split out of
//! `fleet_cleanup.rs` so that file stays under the 300-line cap
//! after the audit follow-up added per-item failure tracking
//! (T828d03c).

use super::OutputMode;
use serde_json::json;
use std::path::PathBuf;

/// Render the sweep result for the chosen output mode.
pub(super) fn render(
    orphan_worktrees: &[PathBuf],
    stale_branches: &[String],
    failures: &[(PathBuf, String)],
    prune_ran: bool,
    output: OutputMode,
    dry_run: bool,
) {
    match output {
        OutputMode::Json => render_json(
            orphan_worktrees,
            stale_branches,
            failures,
            prune_ran,
            dry_run,
        ),
        OutputMode::Plain => render_plain(orphan_worktrees, stale_branches, failures),
        OutputMode::Human => render_human(
            orphan_worktrees,
            stale_branches,
            failures,
            prune_ran,
            dry_run,
        ),
    }
}

fn render_json(
    orphan_worktrees: &[PathBuf],
    stale_branches: &[String],
    failures: &[(PathBuf, String)],
    prune_ran: bool,
    dry_run: bool,
) {
    let v = json!({
        "dry_run": dry_run,
        "orphan_worktrees": orphan_worktrees.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        "stale_branches":   stale_branches,
        "failures":         failures.iter().map(|(p, e)| json!({"path": p.display().to_string(), "error": e})).collect::<Vec<_>>(),
        "prune_ran":        prune_ran,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&v).unwrap_or_else(|_| "{}".into())
    );
}

fn render_plain(
    orphan_worktrees: &[PathBuf],
    stale_branches: &[String],
    failures: &[(PathBuf, String)],
) {
    for p in orphan_worktrees {
        println!("worktree {}", p.display());
    }
    for b in stale_branches {
        println!("branch {b}");
    }
    for (p, e) in failures {
        println!("failed {}\t{e}", p.display());
    }
}

fn render_human(
    orphan_worktrees: &[PathBuf],
    stale_branches: &[String],
    failures: &[(PathBuf, String)],
    prune_ran: bool,
    dry_run: bool,
) {
    let prefix = if dry_run { "would remove" } else { "removed" };
    println!(
        "cvg fleet cleanup — {} {} orphan worktree(s), {} stale branch(es), {} failure(s).",
        prefix,
        orphan_worktrees.len(),
        stale_branches.len(),
        failures.len(),
    );
    for p in orphan_worktrees {
        println!("  worktree  {}", p.display());
    }
    for b in stale_branches {
        println!("  branch    {b}");
    }
    for (p, e) in failures {
        println!("  ! failed  {}: {e}", p.display());
    }
    if !dry_run && prune_ran {
        println!("  (git worktree prune ran)");
    }
    println!("  note: agent_processes rows in state.db are reconciled by the daemon reaper.");
}
