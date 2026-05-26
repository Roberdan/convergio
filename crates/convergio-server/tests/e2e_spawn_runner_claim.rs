//! `/v1/agents/spawn-runner` atomic-claim regression (W1-C, 2026-05-12).
//!
//! Pre-2026-05-12 the route spawned the OS process BEFORE
//! transitioning the task. Two concurrent spawn-runner calls for the
//! same task therefore both succeeded at spawning, then raced at the
//! transition step — leaving one runner attached to an unclaimed
//! task. Audit `convergio-server/src/routes/agents.rs:127` (HIGH).
//!
//! This file exercises the new contract:
//!   1. First spawn-runner: 200 + task moves to `in_progress`.
//!   2. Second spawn-runner against the same task: 400
//!      `claim_conflict` (NOT a successful spawn).

mod common;

use convergio_durability::NewTask;
use reqwest::Client;
use serde_json::{json, Value};

#[tokio::test]
async fn second_spawn_runner_on_same_task_is_refused_with_claim_conflict() {
    let (base, pool, _dir) = common::boot().await;
    let client = Client::new();

    let durability = convergio_durability::Durability::new(pool.clone());
    let plan = durability
        .create_plan(convergio_durability::NewPlan {
            title: "p".into(),
            description: None,
            project: None,
            no_dispatch_default: false,
        })
        .await
        .unwrap();
    let task = durability
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "t".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
                no_dispatch: None,
            },
        )
        .await
        .unwrap();

    let body = json!({
        "agent_id": "shell-runner-01",
        "kind": "shell",
        "command": "/bin/sleep",
        "args": ["0"],
        "plan_id": plan.id,
        "task_id": task.id,
        "capabilities": [],
    });

    // First call wins the claim.
    let first = client
        .post(format!("{base}/v1/agents/spawn-runner"))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(first.status(), 200, "first spawn must succeed");
    let first_body: Value = first.json().await.unwrap();
    assert_eq!(first_body["task"]["status"], "in_progress");
    assert_eq!(first_body["task"]["agent_id"], "shell-runner-01");

    // Second call sees the task already claimed — must NOT spawn a
    // second process and must NOT silently succeed.
    let second_body = json!({
        "agent_id": "shell-runner-02",
        "kind": "shell",
        "command": "/bin/sleep",
        "args": ["0"],
        "plan_id": plan.id,
        "task_id": task.id,
        "capabilities": [],
    });
    let second = client
        .post(format!("{base}/v1/agents/spawn-runner"))
        .json(&second_body)
        .send()
        .await
        .unwrap();
    assert_eq!(
        second.status(),
        400,
        "second spawn must be refused with HTTP 400"
    );
    let err: Value = second.json().await.unwrap();
    assert_eq!(
        err["error"]["code"], "claim_conflict",
        "stable error code for claim refusal"
    );

    // The first agent still owns the task; the second was NOT
    // registered (no agent process for shell-runner-02).
    let final_task = durability.tasks().get(&task.id).await.unwrap();
    assert_eq!(final_task.status.as_str(), "in_progress");
    assert_eq!(final_task.agent_id.as_deref(), Some("shell-runner-01"));
}

#[tokio::test]
async fn spawn_runner_without_task_id_skips_the_claim_check() {
    let (base, _pool, _dir) = common::boot().await;
    let client = Client::new();

    // No task_id => no claim. Process should spawn normally.
    let resp = client
        .post(format!("{base}/v1/agents/spawn-runner"))
        .json(&json!({
            "agent_id": "shell-untasked-01",
            "kind": "shell",
            "command": "/bin/sleep",
            "args": ["0"],
            "capabilities": [],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body["task"].is_null(), "no task carried back");
    assert_eq!(body["agent"]["id"], "shell-untasked-01");
}
