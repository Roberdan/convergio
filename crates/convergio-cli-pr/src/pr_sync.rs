//! `cvg pr sync <plan-id>` — auto-transition pending plan tasks to
//! `submitted` when their tracking PR has merged.
//!
//! Reads merged GitHub PRs via `gh pr list --state merged`, parses each
//! body for `Tracks: <task-uuid>` lines (one or more, comma- or
//! whitespace-separated), and POSTs `submitted` transitions to the
//! daemon for tasks belonging to the named plan that are still
//! `pending`. Tasks already `submitted` / `done` / `failed` are skipped.
//!
//! Evidence injection is **not** done in v1 — the daemon's
//! [`EvidenceGate`] still applies. If a task requires evidence and none
//! is attached, the transition is reported as `failed` with the gate
//! reason. The operator (or a follow-up version) attaches evidence
//! before re-running. This is the structural fix for friction-log F35
//! and the v0.2.x finishing-line task **T2.04**.
//!
//! Convention: PR authors add a `Tracks:` line to the PR body for every
//! task this PR closes. See `.github/pull_request_template.md`.

use super::pr_link::detect_repo_slug_or_unknown;
use super::pr_sync_parse::parse_tracks_lines;
use super::{Client, OutputMode};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::process::Command;

/// Walk merged GitHub PRs, find tasks declared via `Tracks:` lines
/// in the body, and transition any `pending` ones in the named plan
/// to `submitted`.
pub async fn run(
    client: &Client,
    plan_id: String,
    agent_id: Option<String>,
    output: OutputMode,
) -> Result<()> {
    // 1. Fetch plan tasks; remember their ids so cross-plan UUIDs in PR
    //    bodies do not silently match the wrong plan.
    let plan_tasks: Vec<Value> = client.get(&format!("/v1/plans/{plan_id}/tasks")).await?;
    let plan_task_ids: BTreeSet<String> = plan_tasks
        .iter()
        .filter_map(|t| t.get("id").and_then(Value::as_str).map(String::from))
        .collect();

    if plan_task_ids.is_empty() {
        return render_report(&SyncReport::default(), output);
    }

    // 2. Pull recent merged PRs.
    let prs = fetch_merged_prs()?;

    // 3. Build (pr_number, task_id) pairs filtered to this plan only.
    let mut tracked: Vec<(i64, String)> = Vec::new();
    for pr in &prs {
        let pr_num = pr.get("number").and_then(Value::as_i64).unwrap_or(0);
        let body = pr.get("body").and_then(Value::as_str).unwrap_or("");
        for task_id in parse_tracks_lines(body) {
            if plan_task_ids.contains(&task_id) {
                tracked.push((pr_num, task_id));
            }
        }
    }

    // 4. Resolve repo slug once (best-effort; needed for plan_pr_links).
    let repo_slug = detect_repo_slug_or_unknown();

    // 5. Transition each in turn and populate plan_pr_links.
    let mut report = SyncReport {
        scanned_prs: prs.len(),
        tracked_pairs: tracked.len(),
        ..SyncReport::default()
    };
    for (pr_num, task_id) in tracked {
        let task: Value = match client.get::<Value>(&format!("/v1/tasks/{task_id}")).await {
            Ok(t) => t,
            Err(e) => {
                report.failed.push(SyncFailure {
                    pr_number: pr_num,
                    task_id: task_id.clone(),
                    reason: format!("fetch task: {e}"),
                });
                continue;
            }
        };
        let status = task.get("status").and_then(Value::as_str).unwrap_or("");
        let task_plan_id = task
            .get("plan_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let task_agent_id = task
            .get("agent_id")
            .and_then(Value::as_str)
            .map(str::to_string);

        // Populate plan_pr_links regardless of task status so the
        // mapping is recorded even for already-closed tasks.
        if !task_plan_id.is_empty() {
            let link_body = json!({
                "pr_number": pr_num,
                "repo_slug": repo_slug,
                "task_id":   task_id,
                "agent_id":  task_agent_id,
            });
            // Non-fatal — if the plan is missing or the daemon is
            // unreachable we log it and continue.
            let _: Result<Value> = client
                .post(&format!("/v1/plans/{task_plan_id}/pr-links"), &link_body)
                .await;
        }

        if matches!(status, "submitted" | "done") {
            report.skipped.push(SyncSkip {
                pr_number: pr_num,
                task_id: task_id.clone(),
                current_status: status.to_string(),
            });
            continue;
        }
        let body = json!({
            "target": "submitted",
            "agent_id": agent_id,
        });
        let result: Result<Value> = client
            .post(&format!("/v1/tasks/{task_id}/transition"), &body)
            .await;
        match result {
            Ok(_) => report.transitioned.push(SyncOk {
                pr_number: pr_num,
                task_id: task_id.clone(),
                previous_status: status.to_string(),
            }),
            Err(e) => report.failed.push(SyncFailure {
                pr_number: pr_num,
                task_id: task_id.clone(),
                reason: e.to_string(),
            }),
        }
    }

    render_report(&report, output)
}

fn fetch_merged_prs() -> Result<Vec<Value>> {
    let out = Command::new("gh")
        .args([
            "pr",
            "list",
            "--state",
            "merged",
            "--limit",
            "50",
            "--json",
            "number,title,body,mergeCommit,mergedAt",
        ])
        .output()
        .context("spawn gh — is the gh CLI installed and authenticated?")?;
    if !out.status.success() {
        anyhow::bail!(
            "gh pr list --state merged failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    serde_json::from_slice(&out.stdout).context("parse gh json output")
}

#[derive(Default)]
struct SyncReport {
    scanned_prs: usize,
    tracked_pairs: usize,
    transitioned: Vec<SyncOk>,
    skipped: Vec<SyncSkip>,
    failed: Vec<SyncFailure>,
}

struct SyncOk {
    pr_number: i64,
    task_id: String,
    previous_status: String,
}

struct SyncSkip {
    pr_number: i64,
    task_id: String,
    current_status: String,
}

struct SyncFailure {
    pr_number: i64,
    task_id: String,
    reason: String,
}

fn render_report(report: &SyncReport, output: OutputMode) -> Result<()> {
    match output {
        OutputMode::Json => {
            let body = json!({
                "scanned_prs": report.scanned_prs,
                "tracked_pairs": report.tracked_pairs,
                "transitioned": report.transitioned.iter().map(|o| json!({
                    "pr_number": o.pr_number,
                    "task_id": o.task_id,
                    "previous_status": o.previous_status,
                })).collect::<Vec<_>>(),
                "skipped": report.skipped.iter().map(|s| json!({
                    "pr_number": s.pr_number,
                    "task_id": s.task_id,
                    "current_status": s.current_status,
                })).collect::<Vec<_>>(),
                "failed": report.failed.iter().map(|f| json!({
                    "pr_number": f.pr_number,
                    "task_id": f.task_id,
                    "reason": f.reason,
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&body)?);
        }
        OutputMode::Plain => {
            println!(
                "scanned={} tracked={} transitioned={} skipped={} failed={}",
                report.scanned_prs,
                report.tracked_pairs,
                report.transitioned.len(),
                report.skipped.len(),
                report.failed.len()
            );
        }
        _ => {
            println!(
                "cvg pr sync — scanned {} merged PRs, {} (PR, task) pairs found",
                report.scanned_prs, report.tracked_pairs
            );
            println!();
            println!(
                "  transitioned ({}):  {} → submitted",
                report.transitioned.len(),
                if report.transitioned.is_empty() {
                    "no tasks"
                } else {
                    "pending"
                }
            );
            for o in &report.transitioned {
                println!("    PR #{} → task {}", o.pr_number, &o.task_id[..8]);
            }
            println!();
            println!(
                "  skipped ({}): already submitted or done",
                report.skipped.len()
            );
            for s in &report.skipped {
                println!(
                    "    PR #{} → task {} ({})",
                    s.pr_number,
                    &s.task_id[..8],
                    s.current_status
                );
            }
            println!();
            println!(
                "  failed ({}): gate refusal or transport error",
                report.failed.len()
            );
            for f in &report.failed {
                println!(
                    "    PR #{} → task {}: {}",
                    f.pr_number,
                    &f.task_id[..8],
                    f.reason
                );
            }
        }
    }
    Ok(())
}

// Pure parser unit tests live in `pr_sync_parse.rs`.
