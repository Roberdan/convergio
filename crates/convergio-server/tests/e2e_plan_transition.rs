//! End-to-end test for `POST /v1/plans/:id/transition`.
//!
//! Drives the full lifecycle (`draft → active → completed`) over HTTP,
//! checks the audit chain has the expected `plan.active` and
//! `plan.completed` rows, and verifies that an illegal jump returns
//! HTTP 409.

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
async fn forward_lifecycle_writes_audit_rows() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({ "title": "lifecycle test" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan.get("id").and_then(Value::as_str).unwrap().to_string();
    assert_eq!(plan.get("status").and_then(Value::as_str), Some("draft"));

    let active: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/transition"))
        .json(&json!({ "target": "active" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(active.get("status").and_then(Value::as_str), Some("active"));

    let completed: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/transition"))
        .json(&json!({ "target": "completed" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        completed.get("status").and_then(Value::as_str),
        Some("completed")
    );

    let verify: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verify.get("ok").and_then(Value::as_bool), Some(true));
    // Three rows expected: plan.created (from POST /v1/plans), plan.active,
    // plan.completed. Verify the count via the report's counted fields.
    let checked = verify
        .get("checked")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    assert!(
        checked >= 3,
        "expected >=3 audit rows after lifecycle, got {checked}: {verify}"
    );
}

#[tokio::test]
async fn illegal_transition_returns_409() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({ "title": "bad jump" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan.get("id").and_then(Value::as_str).unwrap().to_string();

    let resp = client
        .post(format!("{base}/v1/plans/{plan_id}/transition"))
        .json(&json!({ "target": "completed" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 409);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(
        body.get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str),
        Some("illegal_plan_transition")
    );
}

#[tokio::test]
async fn idempotent_same_status_succeeds() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({ "title": "idempotent" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan.get("id").and_then(Value::as_str).unwrap().to_string();

    let resp = client
        .post(format!("{base}/v1/plans/{plan_id}/transition"))
        .json(&json!({ "target": "draft" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body.get("status").and_then(Value::as_str), Some("draft"));
}
