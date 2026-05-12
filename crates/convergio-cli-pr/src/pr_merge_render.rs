//! Output renderers for `cvg pr merge`.
//!
//! Split out of `pr_merge.rs` so the orchestrator stays under the
//! 300-line per-file cap (CONSTITUTION § Agent context budget;
//! audit finding LOW pr_merge.rs:271).

use super::pr_merge::MergeReport;
use super::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;

pub(super) fn render_report(
    bundle: &Bundle,
    report: &MergeReport,
    output: OutputMode,
    refused: bool,
) -> Result<()> {
    match output {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(report)?);
        }
        OutputMode::Plain => render_plain(report),
        OutputMode::Human => render_human(bundle, report, refused),
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

fn render_human(bundle: &Bundle, r: &MergeReport, refused: bool) {
    let pr = r.pr.to_string();
    println!(
        "{}",
        bundle.t("pr-merge-header", &[("pr", &pr), ("head", &r.head_ref)])
    );
    for c in &r.eight_check {
        println!("  [{}] {}", if c.ok { "ok" } else { "x" }, c.name);
    }
    if refused {
        println!();
        println!("  {}", bundle.t("pr-merge-refused", &[]));
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
    let count = r.tracked_tasks.len().to_string();
    println!(
        "  {}",
        bundle.t("pr-merge-tracked-header", &[("count", &count)])
    );
    for t in &r.tracked_tasks {
        println!("    {}", t);
    }
    if !r.failed_evidence.is_empty() {
        let fcount = r.failed_evidence.len().to_string();
        println!(
            "  {}",
            bundle.t("pr-merge-failed-evidence-header", &[("count", &fcount)])
        );
        for f in &r.failed_evidence {
            println!("    {}", f);
        }
    }
    let note_prefix = bundle.t("pr-merge-note-prefix", &[]);
    for n in &r.notes {
        println!("  {note_prefix} {}", n);
    }
}
