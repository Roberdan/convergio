//! E2E coverage for `GET /v1/audit/events/:seq/compensate`.
//!
//! Boots the server in-process, produces a real `task.completed_by_thor`
//! event via `/v1/plans/:id/validate`, then compensates it and confirms
//! the task reverts to `submitted`.

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
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();

    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
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
async fn compensate_reverts_thor_completed_task_to_submitted() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "compensate plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "needs undo", "evidence_required": ["test_pass"]}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();

    let _: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "agent-comp"}))
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
            "exit_code": 0
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let _: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "submitted", "agent_id": "agent-comp"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let verdict: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verdict["verdict"], "pass", "validate: {verdict}");

    let task_after: Value = client
        .get(format!("{base}/v1/tasks/{task_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(task_after["status"], "done");

    let events: Value = client
        .get(format!("{base}/v1/audit/events?after_seq=0&limit=1000"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let seq = events
        .as_array()
        .unwrap()
        .iter()
        .find(|ev| ev["transition"] == "task.completed_by_thor" && ev["entity_id"] == task_id)
        .and_then(|ev| ev["seq"].as_i64())
        .expect("expected to find task.completed_by_thor event");

    // Dry-run: compute the compensating action without applying it.
    let resp: Value = client
        .get(format!("{base}/v1/audit/events/{seq}/compensate"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["source_seq"], seq);
    assert_eq!(resp["source_transition"], "task.completed_by_thor");
    assert_eq!(resp["applied"], false);

    let task_after: Value = client
        .get(format!("{base}/v1/tasks/{task_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(task_after["status"], "done");

    // Apply: execute the compensation.
    let resp: Value = client
        .get(format!(
            "{base}/v1/audit/events/{seq}/compensate?apply=true"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(resp["source_seq"], seq);
    assert_eq!(resp["applied"], true);

    let task_after: Value = client
        .get(format!("{base}/v1/tasks/{task_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(task_after["status"], "submitted");

    let events_after: Value = client
        .get(format!("{base}/v1/audit/events?after_seq=0&limit=1000"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(events_after
        .as_array()
        .unwrap()
        .iter()
        .any(|ev| ev["transition"] == "task.reopened" && ev["entity_id"] == task_id));

    let report: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true, "audit chain: {report}");
}
