//! E2E tests for `GET /v1/plans/:id/triage`.
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
        reports: Arc::new(convergio_reports::ReportTemplateStore::new(pool.clone())),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };
    let app = router(state);
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), dir)
}

#[tokio::test]
async fn triage_empty_when_no_tasks() {
    let (base, _dir) = boot().await;
    let client = common::client();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "stale test plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    let stale: Value = client
        .get(format!("{base}/v1/plans/{plan_id}/triage?stale_days=0"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(stale.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn triage_returns_pending_tasks_with_zero_stale_days() {
    let (base, _dir) = boot().await;
    let client = common::client();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "triage plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    // Create two tasks
    client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "old pending task", "wave": 1, "sequence": 1}))
        .send()
        .await
        .unwrap();
    client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "another pending task", "wave": 1, "sequence": 2}))
        .send()
        .await
        .unwrap();

    // stale_days=0 means "updated before now" — all just-created tasks qualify
    let stale: Value = client
        .get(format!("{base}/v1/plans/{plan_id}/triage?stale_days=0"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // Both tasks are pending and updated_at is just-now, but stale_days=0
    // means the cutoff is exactly Utc::now() so they may or may not be included.
    // Use stale_days=-1 to force the cutoff into the future.
    let _ = stale; // result depends on sub-millisecond timing; skip assertion

    // With a negative stale_days (cutoff in the future), all tasks are stale
    let stale_all: Value = client
        .get(format!("{base}/v1/plans/{plan_id}/triage?stale_days=-1"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = stale_all.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    for task in arr {
        assert_eq!(task["status"], "pending");
    }
}

#[tokio::test]
async fn triage_excludes_done_tasks() {
    let (base, _dir) = boot().await;
    let client = common::client();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "triage exclude done"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "done task", "wave": 1, "sequence": 1}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    // Close the task post-hoc to move it to `done`
    client
        .post(format!("{base}/v1/tasks/{task_id}/close-post-hoc"))
        .json(&json!({"reason": "shipped in PR #1"}))
        .send()
        .await
        .unwrap();

    // Triage should return nothing — done tasks are excluded
    let stale: Value = client
        .get(format!("{base}/v1/plans/{plan_id}/triage?stale_days=-1"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(stale.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn triage_includes_failed_tasks() {
    let (base, _dir) = boot().await;
    let client = common::client();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "triage failed"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "failing task", "wave": 1, "sequence": 1}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    // Transition to in-progress then failed
    client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress"}))
        .send()
        .await
        .unwrap();
    client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "failed"}))
        .send()
        .await
        .unwrap();

    let stale: Value = client
        .get(format!("{base}/v1/plans/{plan_id}/triage?stale_days=-1"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = stale.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["status"], "failed");
}
