//! E2E coverage for `DELETE /v1/evidence/:id`.
//!
//! Exercises the retroactive cleanup path: attach evidence, delete it,
//! confirm the task evidence list is empty again, and verify the audit
//! chain captures both `evidence.attached` and `evidence.removed`.

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
async fn delete_evidence_removes_row_and_audits_event() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "evidence delete plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "task with retroactive cleanup"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();

    let attached: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "code",
            "payload": {"note": "pre-fix"},
            "exit_code": 0,
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let evidence_id = attached["id"].as_str().unwrap().to_string();

    // Round-trip: list shows the row.
    let list: Value = client
        .get(format!("{base}/v1/tasks/{task_id}/evidence"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(list.as_array().unwrap().len(), 1);

    // DELETE returns the removed row.
    let removed: Value = client
        .delete(format!("{base}/v1/evidence/{evidence_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(removed["id"], evidence_id);
    assert_eq!(removed["kind"], "code");

    // List is now empty.
    let list: Value = client
        .get(format!("{base}/v1/tasks/{task_id}/evidence"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(list.as_array().unwrap().is_empty());

    // A second DELETE returns 404.
    let resp = client
        .delete(format!("{base}/v1/evidence/{evidence_id}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // Audit chain still verifies and counts both `evidence.attached`
    // and `evidence.removed` (plus plan.created + task.created).
    let report: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true, "audit chain: {report}");
    assert!(
        report["checked"].as_i64().unwrap() >= 4,
        "expected 4+ events (plan.created, task.created, evidence.attached, evidence.removed): {report}"
    );
}

#[tokio::test]
async fn delete_unknown_evidence_returns_404() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();
    let resp = client
        .delete(format!(
            "{base}/v1/evidence/00000000-0000-0000-0000-000000000000"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
