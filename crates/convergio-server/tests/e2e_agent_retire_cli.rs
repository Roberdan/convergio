//! E2E: heartbeat with `status="retired"` returns 422 with the
//! clearer message that points operators at the dedicated retire
//! route (closes C4 from the 2026-05-04 retro).
//!
//! Also asserts the explicit retire endpoint still works alongside
//! the heartbeat alias rejection.

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::init;
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
    let pool = Pool::connect(&format!("sqlite://{}", db_path.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    let state = AppState {
        durability: Arc::new(convergio_durability::Durability::new(pool.clone())),
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
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    (format!("http://{addr}"), dir)
}

#[tokio::test]
async fn heartbeat_with_retired_status_returns_clearer_422() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    client
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({
            "id": "subagent-retire-test",
            "kind": "subagent",
            "name": "retire heartbeat smoke",
            "host": "macOS",
            "capabilities": ["test"]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let resp = client
        .post(format!(
            "{base}/v1/agent-registry/agents/subagent-retire-test/heartbeat"
        ))
        .json(&json!({"status": "retired"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("retire"),
        "422 message must mention retire endpoint, got: {msg}"
    );
    assert!(
        msg.contains("/retire") || msg.contains("retire instead"),
        "422 message must point at the retire route, got: {msg}"
    );

    // The explicit retire endpoint still works.
    let retired: Value = client
        .post(format!(
            "{base}/v1/agent-registry/agents/subagent-retire-test/retire"
        ))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(retired["status"], "retired");
}

#[tokio::test]
async fn heartbeat_with_unknown_status_still_returns_422() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();
    client
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({
            "id": "subagent-bad-status",
            "kind": "subagent",
            "name": "bad heartbeat status",
            "host": "macOS",
            "capabilities": ["test"]
        }))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let resp = client
        .post(format!(
            "{base}/v1/agent-registry/agents/subagent-bad-status/heartbeat"
        ))
        .json(&json!({"status": "garbage"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    let body: Value = resp.json().await.unwrap();
    let msg = body["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("garbage") || msg.contains("unknown agent status"),
        "422 message must explain the bad status, got: {msg}"
    );
}
