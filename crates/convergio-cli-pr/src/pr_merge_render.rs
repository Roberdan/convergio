//! Output renderers for `cvg pr merge`.
//!
//! Split out of `pr_merge.rs` so the orchestrator stays under the
//! 300-line per-file cap (CONSTITUTION § Agent context budget;
//! audit finding LOW pr_merge.rs:271).

use super::pr_merge::MergeReport;
use super::OutputMode;
use anyhow::Result;

pub(super) fn render_report(report: &MergeReport, output: OutputMode, refused: bool) -> Result<()> {
    match output {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(report)?);
        }
        OutputMode::Plain => render_plain(report),
        OutputMode::Human => render_human(report, refused),
    }
    Ok(())
}

fn render_plain(r: &MergeReport) {
    println!(
        "pr={} merged={} auto_resolved={} worktree={} branch={} agent_retired={} tasks={} failed_evidence={}",
        r.pr,
        r.merged,
        r.auto_conflict_resolved,
        r.worktree_removed.is_some(),
        r.local_branch_deleted,
        r.agent_retired.as_deref().unwrap_or("-"),
        r.tracked_tasks.len(),
        r.failed_evidence.len()
    );
}

fn render_human(r: &MergeReport, refused: bool) {
    println!("cvg pr merge — PR #{} ({})", r.pr, r.head_ref);
    for c in &r.eight_check {
        println!("  [{}] {}", if c.ok { "ok" } else { "x" }, c.name);
    }
    if refused {
        println!("\n  refused: 4-check did not pass.");
        return;
    }
    println!(
        "\n  merged: {} | auto-resolved: {} | worktree: {} | branch L/R: {}/{} | agent retired: {}",
        r.merged,
        r.auto_conflict_resolved,
        r.worktree_removed.as_deref().unwrap_or("-"),
        r.local_branch_deleted,
        r.remote_branch_deleted,
        r.agent_retired.as_deref().unwrap_or("-")
    );
    println!("  tracked tasks updated ({}):", r.tracked_tasks.len());
    for t in &r.tracked_tasks {
        println!("    {}", t);
    }
    if !r.failed_evidence.is_empty() {
        println!(
            "  failed evidence writes ({}): merge_record was NOT attached",
            r.failed_evidence.len()
        );
        for f in &r.failed_evidence {
            println!("    {}", f);
        }
    }
    for n in &r.notes {
        println!("  note: {}", n);
    }
}
