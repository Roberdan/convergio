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
use super::pr_sync_render::render_report;
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
        // mapping is recorded even for already-closed tasks. The
        // result is recorded via [`record_link_attempt`] so POST
        // failures surface in the report instead of being dropped
        // (audit finding LOW pr_sync.rs:107).
        if !task_plan_id.is_empty() {
            let link_body = json!({
                "pr_number": pr_num,
                "repo_slug": repo_slug,
                "task_id":   task_id,
                "agent_id":  task_agent_id,
            });
            let link_result: Result<Value> = client
                .post(&format!("/v1/plans/{task_plan_id}/pr-links"), &link_body)
                .await;
            record_link_attempt(&mut report, pr_num, &task_id, &link_result);
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
pub(super) struct SyncReport {
    pub(super) scanned_prs: usize,
    pub(super) tracked_pairs: usize,
    pub(super) transitioned: Vec<SyncOk>,
    pub(super) skipped: Vec<SyncSkip>,
    pub(super) failed: Vec<SyncFailure>,
    /// `plan_pr_links` POST failures recorded per (PR, task) pair.
    /// The pr_sync.rs:107 audit finding flagged that these were
    /// silently dropped despite the comment claiming they were
    /// logged. Surfaced in every render mode.
    pub(super) link_failures: Vec<SyncFailure>,
}

pub(super) struct SyncOk {
    pub(super) pr_number: i64,
    pub(super) task_id: String,
    pub(super) previous_status: String,
}

pub(super) struct SyncSkip {
    pub(super) pr_number: i64,
    pub(super) task_id: String,
    pub(super) current_status: String,
}

pub(super) struct SyncFailure {
    pub(super) pr_number: i64,
    pub(super) task_id: String,
    pub(super) reason: String,
}

// Pure parser unit tests live in `pr_sync_parse.rs`.

/// Record the outcome of a `plan_pr_links` POST against the sync
/// report. Errors push into `link_failures` so every render mode
/// can surface them; success is a no-op. Audit finding LOW
/// pr_sync.rs:107.
fn record_link_attempt(
    report: &mut SyncReport,
    pr_number: i64,
    task_id: &str,
    result: &Result<Value>,
) {
    if let Err(e) = result {
        report.link_failures.push(SyncFailure {
            pr_number,
            task_id: task_id.to_string(),
            reason: format!("{e}"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Audit finding (LOW, pr_sync.rs:107): `plan_pr_links` POST
    // failures are silently discarded even though the surrounding
    // comment says they are logged. The fix records each failure in
    // `link_failures` so JSON / plain / human output all surface it.
    #[test]
    fn record_link_attempt_pushes_failure_into_report() {
        let mut report = SyncReport::default();
        let err: Result<Value> = Err(anyhow::anyhow!("HTTP 502 from /pr-links"));
        record_link_attempt(&mut report, 12, "task-abc", &err);
        assert_eq!(
            report.link_failures.len(),
            1,
            "POST failure must be recorded so cvg pr sync stops hiding link errors"
        );
        let f = &report.link_failures[0];
        assert_eq!(f.pr_number, 12);
        assert_eq!(f.task_id, "task-abc");
        assert!(f.reason.contains("502"));
    }

    #[test]
    fn record_link_attempt_ignores_success() {
        let mut report = SyncReport::default();
        let ok: Result<Value> = Ok(json!({}));
        record_link_attempt(&mut report, 1, "task-1", &ok);
        assert!(report.link_failures.is_empty());
    }

    #[test]
    fn link_failures_are_surfaced_in_json_render() {
        let report = SyncReport {
            link_failures: vec![SyncFailure {
                pr_number: 7,
                task_id: "task-7".into(),
                reason: "boom".into(),
            }],
            ..SyncReport::default()
        };
        let body = super::super::pr_sync_render::report_json(&report);
        assert!(body
            .get("link_failures")
            .and_then(|v| v.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false));
    }
}
