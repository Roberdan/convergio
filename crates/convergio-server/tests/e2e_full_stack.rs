//! Cross-layer end-to-end test.
//!
//! Drives every layer through HTTP in a single workflow:
//!
//! 1. Layer 1: create plan, create task with required evidence
//! 2. Layer 3: spawn an agent process to handle the task
//! 3. Layer 1: claim the task with that agent_id
//! 4. Layer 2: agent publishes a `task.progress` message on the bus
//! 5. Layer 1: attach evidence
//! 6. Layer 2: a consumer polls and acks
//! 7. Layer 1: transition to submitted (gate must allow now)
//! 8. Layer 1: verify the audit chain — every step above wrote a row
mod common;

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

async fn boot() -> (String, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    let url = format!("sqlite://{}", db_path.display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_ops::init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();

    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
        ops: Arc::new(convergio_ops::Ops::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
        fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
        ontology: Arc::new(convergio_ontology::Store::new(pool.clone())),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };
    let app = router(state);

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{addr}"), dir)
}

#[tokio::test]
async fn three_layers_cooperate_in_one_workflow() {
    let (base, _dir) = boot().await;
    let c = common::client();

    // 1. Create plan.
    let plan: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "cross-layer"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();

    // 1. Create task with required evidence.
    let task: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({
            "title": "do work",
            "evidence_required": ["completed"],
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();

    // 2. Layer 3: spawn an agent process.
    let agent: Value = c
        .post(format!("{base}/v1/agents/spawn"))
        .json(&json!({
            "kind": "shell",
            "command": "/bin/echo",
            "args": ["working"],
            "env": [],
            "plan_id": plan_id,
            "task_id": task_id,
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let agent_id = agent["id"].as_str().unwrap().to_string();
    assert_eq!(agent["status"], "running");

    // 3. Claim the task as that agent.
    let _: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": agent_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // 4. Agent publishes progress.
    let msg: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/messages"))
        .json(&json!({
            "topic": "task.progress",
            "sender": agent_id,
            "payload": {"task_id": task_id, "step": "halfway"},
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let msg_id = msg["id"].as_str().unwrap().to_string();

    // 5. Attach evidence.
    let _: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "completed",
            "payload": {"output": "done"},
            "exit_code": 0,
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // 6. Another consumer polls + acks.
    let polled: Vec<Value> = c
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages?topic=task.progress&limit=10"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(polled.len(), 1);
    assert_eq!(polled[0]["id"], msg_id);

    let _: Value = c
        .post(format!("{base}/v1/messages/{msg_id}/ack"))
        .json(&json!({"consumer": "watcher"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // 7. Transition to submitted — gate now allows.
    let submitted: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "submitted", "agent_id": agent_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(submitted["status"], "submitted");

    // 8. Audit chain still verifies. We expect at least:
    //    plan.created, task.created, task.in_progress,
    //    evidence.attached, task.submitted (5 rows)
    //
    //    The bus message and agent spawn are NOT (yet) audited —
    //    sessione 4+ may add audit hooks for them. The chain integrity
    //    of what IS audited must hold regardless.
    let report: Value = c
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true, "audit chain integrity: {report}");
    assert!(report["checked"].as_i64().unwrap() >= 5);

    // Sanity: the task is visible with its final state.
    let task_after: Value = c
        .get(format!("{base}/v1/tasks/{task_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(task_after["status"], "submitted");
    assert_eq!(task_after["agent_id"], agent_id);

    // Sanity: the agent process row is still queryable.
    let agent_after: Value = c
        .get(format!("{base}/v1/agents/{agent_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(agent_after["id"], agent_id);
}
