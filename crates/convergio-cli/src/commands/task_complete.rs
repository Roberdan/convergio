//! Orchestration logic for `cvg task complete`.
//!
//! Sequence: graph for-task --semantic → embed for-task → evidence add ×N
//!           → transition submitted → validate plan (Thor) → done.

use super::Client;
use anyhow::{bail, Context, Result};
use convergio_i18n::Bundle;
use serde_json::{json, Value};
use std::process::Command;

/// Run the full completion orchestration for one task.
///
/// Drives the task from its current state through the standard
/// completion pipeline: context retrieval → evidence attachment →
/// gate-checked submit → Thor validation → done.
pub(super) async fn run(
    client: &Client,
    bundle: &Bundle,
    task_id: &str,
    pr: u64,
    agent_id: Option<&str>,
) -> Result<Value> {
    // 1. Fetch task for plan_id and title.
    let task: Value = client
        .get(&format!("/v1/tasks/{task_id}"))
        .await
        .context("fetching task")?;
    let plan_id = task["plan_id"]
        .as_str()
        .context("task missing plan_id")?
        .to_string();
    let title = task["title"].as_str().unwrap_or("task").to_string();

    // 2. Graph context pack (best-effort, semantic fusion).
    eprintln!("{}", bundle.t("task-complete-step-graph", &[]));
    let pack: Value = client
        .get(&format!(
            "/v1/graph/for-task/{task_id}?semantic=1&semantic_top_k=25"
        ))
        .await
        .unwrap_or_else(|_| json!({"ok": false, "skipped": true}));

    // 3. Semantic embed query (best-effort).
    eprintln!("{}", bundle.t("task-complete-step-embed", &[]));
    let embed_hits: Value = client
        .post("/v1/embed/for-task", &json!({"query": title, "top_k": 10}))
        .await
        .unwrap_or_else(|_| json!({"ok": false, "skipped": true}));

    // 4. Evidence: graph_pack.
    eprintln!("{}", bundle.t("task-complete-step-evidence-graph", &[]));
    let _: Value = client
        .post(
            &format!("/v1/tasks/{task_id}/evidence"),
            &json!({
                "kind": "graph_pack",
                "payload": summarise_pack(&pack),
            }),
        )
        .await
        .context("adding graph_pack evidence")?;

    // 5. Evidence: embed_query.
    eprintln!("{}", bundle.t("task-complete-step-evidence-embed", &[]));
    let _: Value = client
        .post(
            &format!("/v1/tasks/{task_id}/evidence"),
            &json!({
                "kind": "embed_query",
                "payload": summarise_embed(&embed_hits),
            }),
        )
        .await
        .context("adding embed_query evidence")?;

    // 6. Evidence: PR reference.
    // Keep both `pr_link` (legacy) and `pr_url` (current) so the EvidenceGate
    // can be satisfied regardless of template.
    let pr_str = pr.to_string();
    eprintln!(
        "{}",
        bundle.t("task-complete-step-evidence-pr", &[("pr", &pr_str)])
    );

    let _: Value = client
        .post(
            &format!("/v1/tasks/{task_id}/evidence"),
            &json!({
                "kind": "pr_link",
                "payload": {"pr": pr},
            }),
        )
        .await
        .context("adding pr_link evidence")?;

    let repo_slug = detect_repo_slug_or_unknown();
    let pr_url = format!("https://github.com/{repo_slug}/pull/{pr}");
    let _: Value = client
        .post(
            &format!("/v1/tasks/{task_id}/evidence"),
            &json!({
                "kind": "pr_url",
                "payload": {"url": pr_url},
            }),
        )
        .await
        .context("adding pr_url evidence")?;

    // Best-effort: register the plan↔PR link so ownership can be resolved.
    let _: Result<Value> = client
        .post(
            &format!("/v1/plans/{plan_id}/pr-links"),
            &json!({
                "pr_number": pr as i64,
                "repo_slug": repo_slug,
                "task_id": task_id,
                "agent_id": agent_id,
            }),
        )
        .await;

    // 7. Transition to submitted.
    eprintln!("{}", bundle.t("task-complete-step-submit", &[]));
    let submitted: Value = client
        .post(
            &format!("/v1/tasks/{task_id}/transition"),
            &json!({
                "target": "submitted",
                "agent_id": agent_id,
            }),
        )
        .await
        .context("transition to submitted")?;
    if submitted["status"].as_str() != Some("submitted") {
        bail!("unexpected status after submit: {}", submitted["status"]);
    }

    // 8. Validate plan (Thor).
    eprintln!("{}", bundle.t("task-complete-step-thor", &[]));
    let verdict: Value = client
        .post(&format!("/v1/plans/{plan_id}/validate"), &json!({}))
        .await
        .context("validate plan")?;
    let v = verdict["verdict"].as_str().unwrap_or("unknown");
    if v != "pass" {
        let pretty = serde_json::to_string_pretty(&verdict).unwrap_or_default();
        bail!(
            "{}",
            bundle.t("task-complete-thor-failed", &[("verdict", &pretty)])
        );
    }

    // 9. Return final task state.
    let done: Value = client
        .get(&format!("/v1/tasks/{task_id}"))
        .await
        .context("fetching final task")?;
    Ok(done)
}

fn summarise_pack(pack: &Value) -> Value {
    json!({
        "matched_nodes": pack["matched_nodes"].as_array().map(|a| a.len()).unwrap_or(0),
        "files": pack["files"].as_array().map(|a| a.len()).unwrap_or(0),
        "estimated_tokens": pack.get("estimated_tokens"),
    })
}

fn detect_repo_slug_or_unknown() -> String {
    detect_repo_slug().unwrap_or_else(|| "unknown/unknown".into())
}

fn detect_repo_slug() -> Option<String> {
    let out = Command::new("gh")
        .args([
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "-q",
            ".nameWithOwner",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

fn summarise_embed(hits: &Value) -> Value {
    json!({
        "hit_count": hits["hits"].as_array().map(|a| a.len()).unwrap_or(0),
        "model": hits.get("model"),
        "ms": hits.get("ms"),
    })
}
