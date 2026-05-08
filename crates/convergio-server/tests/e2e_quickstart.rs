//! Quickstart E2E — proves the README's "60-second" claim.
//!
//! Pipeline:
//! 1. POST /v1/solve — turn a mission into a plan
//! 2. POST /v1/dispatch — executor moves wave 1 tasks to in_progress
//!    via Layer 3 spawn
//! 3. Attach minimal evidence + transition every task to submitted
//!    (exercise the gate pipeline over HTTP)
//! 4. POST /v1/plans/:id/validate — Thor returns Pass and promotes
//!    submitted → done
//! 5. GET /v1/audit/verify — audit chain still verifies

mod common;

use common::boot as common_boot;
use serde_json::{json, Value};

async fn boot() -> (String, tempfile::TempDir) {
    // Force the deterministic line-split planner so the E2E does
    // not invoke the operator's local `claude -p --model opus`
    // (ADR-0036) — that would charge real tokens on each run.
    std::env::set_var("CONVERGIO_PLANNER_MODE", "heuristic");
    let (base, _pool, dir) = common_boot().await;
    (base, dir)
}

async fn attach_quickstart_evidence(client: &reqwest::Client, base: &str, task_id: &str) {
    // This mirrors the "cvg task complete --pr" evidence trio. It is
    // intentionally lightweight (no real graph/embed content required)
    // because the demo's purpose is to prove the workflow wiring.
    let _ev1: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "graph_pack",
            "payload": {
                "matched_nodes": 0,
                "files": 0,
                "estimated_tokens": 0
            }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let _ev2: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "embed_query",
            "payload": {
                "hit_count": 0
            }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let _ev3: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "pr_link",
            "payload": {"pr": 1}
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
}

#[tokio::test]
async fn solve_dispatch_submit_validate_full_loop() {
    let (base, _dir) = boot().await;
    let c = reqwest::Client::new();

    // 1. Solve a mission.
    let solved: Value = c
        .post(format!("{base}/v1/solve"))
        .json(&json!({"mission": "ship convergio v3\nwrite the demo\nopen-source it"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = solved["plan_id"].as_str().unwrap().to_string();

    // The plan now has 3 tasks in wave 1.
    let tasks_before: Vec<Value> = c
        .get(format!("{base}/v1/plans/{plan_id}/tasks"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tasks_before.len(), 3);

    // 2. Dispatch — executor moves them to in_progress and spawns
    //    /bin/echo for each.
    let dispatch: Value = c
        .post(format!("{base}/v1/dispatch"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(dispatch["dispatched"], 3);

    // Re-fetch so we see post-dispatch status/agent assignment.
    let mut tasks: Vec<Value> = c
        .get(format!("{base}/v1/plans/{plan_id}/tasks"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tasks.len(), 3);

    // Submit in stable order (sequence gate can enforce ordering).
    tasks.sort_by_key(|t| t.get("sequence").and_then(|v| v.as_i64()).unwrap_or(0));

    // 3. Attach minimal evidence and transition every task to submitted.
    for t in &tasks {
        let task_id = t["id"].as_str().unwrap();
        attach_quickstart_evidence(&c, &base, task_id).await;

        let agent_id = t.get("agent_id").and_then(|v| v.as_str());
        let submitted: Value = c
            .post(format!("{base}/v1/tasks/{task_id}/transition"))
            .json(&json!({
                "target": "submitted",
                "agent_id": agent_id,
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        assert_eq!(submitted["status"], "submitted", "submitted: {submitted}");
    }

    // 4. Validate — Thor returns Pass and promotes submitted → done.
    let verdict: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verdict["verdict"], "pass", "verdict: {verdict}");

    // Verify: every task is done now.
    let tasks_after: Vec<Value> = c
        .get(format!("{base}/v1/plans/{plan_id}/tasks"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tasks_after.len(), 3);
    for t in &tasks_after {
        assert_eq!(t["status"], "done", "task not done: {t}");
    }

    // 5. Sanity: the audit chain still verifies.
    let report: Value = c
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true);
}

#[tokio::test]
async fn validate_returns_fail_on_open_tasks() {
    let (base, _dir) = boot().await;
    let c = reqwest::Client::new();

    let solved: Value = c
        .post(format!("{base}/v1/solve"))
        .json(&json!({"mission": "alpha\nbeta"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = solved["plan_id"].as_str().unwrap();

    let verdict: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verdict["verdict"], "fail");
    assert!(
        verdict["reasons"].as_array().unwrap().len() >= 2,
        "verdict: {verdict}"
    );
}
