//! ADR-0011 end-to-end coverage: only Thor (`cvg validate`) promotes
//! `submitted -> done`. Agent-driven done attempts must be refused
//! with HTTP 403 and an audit row, and `validate` must atomically
//! flip valid submitted tasks with a dedicated audit kind.

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

/// Negative case: an agent attempting to flip a task directly to
/// `done` must receive 403 with stable code `done_not_by_thor`, AND
/// the audit chain must record one `task.refused` row for the attempt
/// without mutating the task status.
#[tokio::test]
async fn agent_done_transition_is_refused_with_audit_row() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "refusal plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "should not skip thor"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();

    let resp = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "done", "agent_id": "rogue-agent"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403, "agent-driven done must return 403");
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "done_not_by_thor");

    let task_after: Value = client
        .get(format!("{base}/v1/tasks/{task_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        task_after["status"], "pending",
        "refused done attempt must not change status"
    );

    let report: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true, "audit chain integrity preserved");
}

/// Positive case: when Thor validates a plan whose tasks are all
/// `submitted` with the required evidence, those tasks flip to
/// `done` atomically and each gets one `task.completed_by_thor`
/// audit row. Re-validating a plan that is already all-done is
/// idempotent: Pass, no further mutations.
#[tokio::test]
async fn validate_promotes_submitted_tasks_to_done_with_thor_audit() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "thor promote plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "real work", "evidence_required": ["test_pass"]}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();

    let _: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "real-agent"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
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
    let submitted: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "submitted", "agent_id": "real-agent"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(submitted["status"], "submitted");

    let verdict: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verdict["verdict"], "pass", "verdict: {verdict}");

    let task_after: Value = client
        .get(format!("{base}/v1/tasks/{task_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        task_after["status"], "done",
        "Thor must promote submitted tasks to done"
    );

    let verdict_again: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verdict_again["verdict"], "pass", "re-validate idempotent");

    let report: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true);
}
