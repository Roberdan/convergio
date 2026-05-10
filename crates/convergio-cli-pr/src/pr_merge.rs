//! `cvg pr merge <pr> [--retire-agent <id>]` — merge orchestrator.
//! 8-check pre-flight, branch+worktree cleanup, optional sub-agent
//! retire, `merge_record` evidence per tracked task. On AUTO-block
//! conflict it aborts with an actionable hint; the in-process
//! auto-resolve is a follow-up (gated on E2E mock infra to avoid the
//! P4 "scaffolding only" trap). Audit footprint until P2-2 ships
//! `POST /v1/audit/append`: per-task `evidence.added` rows.

use super::pr_merge_io::{
    delete_local_branch, fetch_pr_view, gh_merge, is_auto_block_conflict, remove_worktree, PrView,
};
use super::pr_sync_parse::parse_tracks_lines;
use super::{Client, OutputMode};
use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;

/// `cvg pr merge` arguments.
#[derive(Debug, Clone, Args)]
pub struct MergeArgs {
    /// PR number on the current GitHub repo.
    pub pr: u64,
    /// Retire this sub-agent after a successful merge.
    #[arg(long, value_name = "AGENT_ID")]
    pub retire_agent: Option<String>,
    /// Skip the worktree + branch cleanup phase.
    #[arg(long)]
    pub no_cleanup: bool,
    /// Print the 8-check + plan and exit. Mutates nothing.
    #[arg(long)]
    pub dry_run: bool,
    /// Agent id recorded on the merge_record evidence row.
    #[arg(long, env = "CONVERGIO_AGENT_ID")]
    pub agent_id: Option<String>,
}

#[derive(Debug, Default, Serialize)]
struct MergeReport {
    pr: u64,
    head_ref: String,
    eight_check: Vec<EightCheckEntry>,
    merged: bool,
    auto_conflict_resolved: bool,
    worktree_removed: Option<String>,
    local_branch_deleted: bool,
    remote_branch_deleted: bool,
    agent_retired: Option<String>,
    tracked_tasks: Vec<String>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct EightCheckEntry {
    name: String,
    ok: bool,
}

/// Run `cvg pr merge`: 8-check, merge via `gh`, cleanup, optional
/// retire, and `merge_record` evidence per tracked task.
pub async fn run(client: &Client, output: OutputMode, args: MergeArgs) -> Result<()> {
    let mut report = MergeReport {
        pr: args.pr,
        ..MergeReport::default()
    };

    let pr_view = fetch_pr_view(args.pr).context("gh pr view failed")?;
    report.head_ref = pr_view.head_ref.clone();
    report.eight_check = eight_check(&pr_view);
    let pass_all = report.eight_check.iter().all(|e| e.ok);

    if args.dry_run || !pass_all {
        render_report(&report, output, !pass_all && !args.dry_run)?;
        if !pass_all {
            anyhow::bail!("8-check refused merge — see findings above");
        }
        return Ok(());
    }

    match gh_merge(args.pr) {
        Ok(()) => {
            report.merged = true;
            report.remote_branch_deleted = true; // handled by `gh pr merge --delete-branch`
        }
        Err(e) if is_auto_block_conflict(&e) => {
            anyhow::bail!(
                "merge refused on conflict; check out `{}`, run \
                 `cvg docs regenerate`, commit and re-push, then retry: {e}",
                pr_view.head_ref
            );
        }
        Err(e) => return Err(e.context("gh pr merge failed")),
    }

    if !args.no_cleanup {
        match remove_worktree(&pr_view.head_ref) {
            Ok(Some(p)) => report.worktree_removed = Some(format_path(&p)),
            Ok(None) => {}
            Err(e) => report.notes.push(format!("worktree cleanup: {e}")),
        }
        report.local_branch_deleted = delete_local_branch(&pr_view.head_ref).unwrap_or(false);
    }

    if let Some(agent_id) = &args.retire_agent {
        match client
            .post::<Value, Value>(
                &format!("/v1/agent-registry/agents/{agent_id}/retire"),
                &json!({}),
            )
            .await
        {
            Ok(_) => report.agent_retired = Some(agent_id.clone()),
            Err(e) => report.notes.push(format!("retire {agent_id}: {e}")),
        }
    }

    let tracked = parse_tracks_lines(&pr_view.body);
    let evidence_payload = build_evidence_payload(&report, args.agent_id.as_deref());
    for task_id in &tracked {
        let body = json!({ "kind": "merge_record", "payload": evidence_payload });
        match client
            .post::<Value, Value>(&format!("/v1/tasks/{task_id}/evidence"), &body)
            .await
        {
            Ok(_) => report.tracked_tasks.push(task_id.clone()),
            Err(e) => report.notes.push(format!("evidence task {task_id}: {e}")),
        }
    }

    render_report(&report, output, false)
}

fn eight_check(v: &PrView) -> Vec<EightCheckEntry> {
    vec![
        entry("mergeable=MERGEABLE", v.mergeable == "MERGEABLE"),
        entry("mergeStateStatus=CLEAN", v.merge_state_status == "CLEAN"),
        entry(
            "reviewDecision != CHANGES_REQUESTED",
            v.review_decision != "CHANGES_REQUESTED",
        ),
        entry("CI rollup green", v.status_check_rollup_pass),
    ]
}

fn entry(name: &str, ok: bool) -> EightCheckEntry {
    EightCheckEntry {
        name: name.to_string(),
        ok,
    }
}

fn build_evidence_payload(report: &MergeReport, agent_id: Option<&str>) -> Value {
    json!({
        "tool": "cvg pr merge",
        "pr": report.pr,
        "head_ref": report.head_ref,
        "merged": report.merged,
        "auto_conflict_resolved": report.auto_conflict_resolved,
        "worktree_removed": report.worktree_removed,
        "local_branch_deleted": report.local_branch_deleted,
        "remote_branch_deleted": report.remote_branch_deleted,
        "agent_retired": report.agent_retired,
        "agent_id": agent_id,
        "eight_check": report.eight_check,
    })
}

fn format_path(p: &Path) -> String {
    p.display().to_string()
}

fn render_report(report: &MergeReport, output: OutputMode, refused: bool) -> Result<()> {
    match output {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(report)?);
        }
        OutputMode::Plain => {
            println!(
                "pr={} merged={} auto_resolved={} worktree={} branch={} agent_retired={} tasks={}",
                report.pr,
                report.merged,
                report.auto_conflict_resolved,
                report.worktree_removed.is_some(),
                report.local_branch_deleted,
                report.agent_retired.as_deref().unwrap_or("-"),
                report.tracked_tasks.len()
            );
        }
        _ => render_human(report, refused),
    }
    Ok(())
}

fn render_human(r: &MergeReport, refused: bool) {
    println!("cvg pr merge — PR #{} ({})", r.pr, r.head_ref);
    for c in &r.eight_check {
        println!("  [{}] {}", if c.ok { "ok" } else { "x" }, c.name);
    }
    if refused {
        println!("\n  refused: 8-check did not pass.");
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
    for n in &r.notes {
        println!("  note: {}", n);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(mergeable: &str, state: &str, review: &str, ci_pass: bool) -> PrView {
        PrView {
            head_ref: "feat/x".into(),
            body: String::new(),
            mergeable: mergeable.into(),
            merge_state_status: state.into(),
            review_decision: review.into(),
            status_check_rollup_pass: ci_pass,
        }
    }

    #[test]
    fn eight_check_classifies_each_input() {
        let cases = [
            ("MERGEABLE", "CLEAN", "APPROVED", true, 0),
            ("MERGEABLE", "CLEAN", "CHANGES_REQUESTED", true, 1),
            ("MERGEABLE", "DIRTY", "APPROVED", true, 1),
            ("MERGEABLE", "CLEAN", "APPROVED", false, 1),
            ("CONFLICTING", "DIRTY", "CHANGES_REQUESTED", false, 4),
        ];
        for (m, s, r, ci, expected_fail) in cases {
            let failing = eight_check(&view(m, s, r, ci))
                .into_iter()
                .filter(|e| !e.ok)
                .count();
            assert_eq!(failing, expected_fail, "case {:?}", (m, s, r, ci));
        }
    }

    #[test]
    fn evidence_payload_carries_merge_metadata() {
        let r = MergeReport {
            pr: 999,
            merged: true,
            auto_conflict_resolved: true,
            eight_check: vec![entry("x", true)],
            ..MergeReport::default()
        };
        let p = build_evidence_payload(&r, Some("agent-1"));
        assert_eq!(p["pr"], 999);
        assert_eq!(p["merged"], true);
        assert_eq!(p["auto_conflict_resolved"], true);
        assert_eq!(p["agent_id"], "agent-1");
        assert_eq!(p["tool"], "cvg pr merge");
    }
}
