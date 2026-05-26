//! Agent registry API E2E tests.
mod common;

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
async fn agent_registry_round_trip_is_audited() {
    let (base, _dir) = boot().await;
    let client = common::client();
    let agent: Value = client
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({
            "id": "agent-a",
            "kind": "copilot",
            "name": "Copilot worker",
            "host": "terminal",
            "capabilities": ["code", "test"],
            "metadata": {"pid": 123}
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(agent["status"], "idle");

    let agent: Value = client
        .post(format!("{base}/v1/agent-registry/agents/agent-a/heartbeat"))
        .json(&json!({"current_task_id": "task-1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(agent["status"], "working");

    let agents: Value = client
        .get(format!("{base}/v1/agent-registry/agents"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(agents.as_array().unwrap().len(), 1);

    let audit: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(audit["ok"], true);
}
