//! CRDT diagnostics and conflict-gate E2E tests.

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
    let url = format!("sqlite://{}", db_path.display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();

    let state = AppState {
        durability: Arc::new(convergio_durability::Durability::new(pool.clone())),
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

fn op(actor_id: &str, counter: i64, task_id: &str, field: &str, crdt: &str, value: Value) -> Value {
    json!({
        "actor_id": actor_id,
        "counter": counter,
        "entity_type": "task",
        "entity_id": task_id,
        "field_name": field,
        "crdt_type": crdt,
        "op_kind": "set",
        "value": value,
        "hlc": format!("2026-04-29T00:00:0{counter}Z/0"),
    })
}

#[tokio::test]
async fn crdt_conflicts_are_listed_and_block_task_submission() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();
    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "crdt api plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();
    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "conflicted task"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    let imported: Value = client
        .post(format!("{base}/v1/crdt/import"))
        .json(&json!({
            "agent_id": "sync-agent",
            "ops": [
                op("actor-a", 1, task_id, "title", "mv_register", json!("alpha")),
                op("actor-b", 1, task_id, "title", "mv_register", json!("beta")),
                op("actor-c", 1, task_id, "description", "mv_register", json!("clean")),
                op("actor-d", 1, task_id, "status_note", "lww_register", json!("ready"))
            ]
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(imported["inserted"], 4);
    assert_eq!(imported["merged_cells"].as_array().unwrap().len(), 3);

    let conflicts: Value = client
        .get(format!("{base}/v1/crdt/conflicts"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(conflicts.as_array().unwrap().len(), 1, "{conflicts}");
    assert_eq!(conflicts[0]["field_name"], "title");

    let resp = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "submitted", "agent_id": "agent-crdt"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "gate_refused");
    assert!(body["error"]["message"]
        .as_str()
        .unwrap()
        .contains("crdt_conflict"));

    let audit: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(audit["ok"], true, "audit: {audit}");
    assert!(audit["checked"].as_i64().unwrap() >= 4);
}
