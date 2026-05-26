//! Workspace-level end-to-end test.
//!
//! Boots the convergio-server router in-process against a tempdir
//! SQLite, drives the full lifecycle of a plan + task + evidence over
//! HTTP, and verifies the audit chain.

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
async fn full_lifecycle_with_audit_verification() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    // Health probe
    let health: Value = client
        .get(format!("{base}/v1/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(health["ok"], true);

    // Create plan
    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "e2e plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();
    assert_eq!(plan["status"], "draft");
    assert_eq!(plan["title"], "e2e plan");

    // Create task with one required evidence kind
    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({
            "title": "ship feature",
            "evidence_required": ["test_pass"],
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();
    assert_eq!(task["status"], "pending");

    // Move to in_progress (no evidence required for this transition)
    let task: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "agent-1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(task["status"], "in_progress");
    assert_eq!(task["agent_id"], "agent-1");

    // Try to submit without evidence — gate should refuse with 409
    let resp = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "submitted"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409, "evidence gate should refuse");
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "gate_refused");

    // Attach the required evidence
    let _: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "test_pass",
            "payload": {"output": "1 passed"},
            "exit_code": 0,
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Now the gate should allow submitted
    let task: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "submitted"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(task["status"], "submitted");

    // Audit chain must verify
    let report: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true, "audit chain should verify: {report}");
    assert!(
        report["checked"].as_i64().unwrap() >= 5,
        "expected 5+ events: plan.created, task.created, task.in_progress, evidence.attached, task.submitted"
    );
    assert_eq!(report["broken_at"], Value::Null);
}

#[tokio::test]
async fn status_summarizes_active_plans_and_completed_tasks() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({
            "title": "status plan",
            "description": "ship the status dashboard",
            "project": "convergio-local",
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();
    assert_eq!(plan["project"], "convergio-local");

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "wire cvg status"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    // Under ADR-0011 the agent must transition through submitted; the
    // validator (Thor) is the only path that flips submitted -> done.
    for target in ["in_progress", "submitted"] {
        let _: Value = client
            .post(format!("{base}/v1/tasks/{task_id}/transition"))
            .json(&json!({"target": target, "agent_id": "agent-status"}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    }
    let verdict: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verdict["verdict"], "pass", "validate verdict: {verdict}");

    let status: Value = client
        .get(format!("{base}/v1/status"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let active = status["active_plans"].as_array().unwrap();
    assert_eq!(active.len(), 1, "status: {status}");
    assert_eq!(active[0]["project"], "convergio-local");
    assert_eq!(active[0]["tasks"]["total"], 1);
    assert_eq!(active[0]["tasks"]["done"], 1);
    assert_eq!(active[0]["description"], "ship the status dashboard");

    let completed_tasks = status["recent_completed_tasks"].as_array().unwrap();
    assert_eq!(completed_tasks.len(), 1, "status: {status}");
    assert_eq!(completed_tasks[0]["title"], "wire cvg status");
    assert_eq!(completed_tasks[0]["plan_title"], "status plan");
}

// ADR-0011 negative + positive cases live in
// `crates/convergio-server/tests/e2e_thor_only_done.rs`.
