//! E2E: ops workflow engine core.
//!
//! Verifies that workflow + instance endpoints are wired end-to-end,
//! persist bitemporal rows, and write audited transitions.

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

const PURPOSE_ID_HEADER: &str = "x-purpose-id";
const TEST_PURPOSE_ID: &str = "00000000-0000-4000-8000-000000000441";

async fn boot() -> (String, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    let url = format!("sqlite://{}", db_path.display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    convergio_ops::init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    let ontology = Arc::new(convergio_ontology::Store::new(pool.clone()));
    ontology.migrate().await.unwrap();

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
        ontology,
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
async fn workflow_instance_can_run_to_completion() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::builder()
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(PURPOSE_ID_HEADER, TEST_PURPOSE_ID.parse().unwrap());
            headers
        })
        .build()
        .unwrap();

    let wf: Value = client
        .post(format!("{base}/v1/ops/workflows"))
        .json(&json!({
            "workflow_key": "test.simple",
            "spec": {
                "start": "start",
                "nodes": [
                    {"id": "start", "kind": {"type": "start"}, "next": ["a1"]},
                    {"id": "a1", "kind": {"type": "action", "name": "do", "input": {}}, "next": ["end"]},
                    {"id": "end", "kind": {"type": "end"}, "next": []}
                ]
            },
            "agent_id": "agent-ops"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let workflow_id = wf["workflow_id"].as_str().unwrap().to_string();
    assert_eq!(wf["workflow_key"], "test.simple");
    assert_eq!(wf["version"], 1);

    let inst: Value = client
        .post(format!("{base}/v1/ops/instances"))
        .json(&json!({
            "workflow_id": workflow_id,
            "context": {"k": true},
            "agent_id": "agent-ops"
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let instance_id = inst["instance_id"].as_str().unwrap().to_string();
    assert_eq!(inst["status"], "running");

    let inst: Value = client
        .post(format!("{base}/v1/ops/instances/{instance_id}/tick"))
        .json(&json!({"agent_id": "agent-ops"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let work_items = inst["state"]["work_items"].as_array().unwrap();
    assert_eq!(work_items.len(), 1);
    let work_item_id = work_items[0]["id"].as_str().unwrap().to_string();

    let _: Value = client
        .post(format!(
            "{base}/v1/ops/instances/{instance_id}/work-items/{work_item_id}/complete"
        ))
        .json(&json!({"success": true, "agent_id": "agent-ops"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let inst: Value = client
        .post(format!("{base}/v1/ops/instances/{instance_id}/tick"))
        .json(&json!({"agent_id": "agent-ops"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(inst["status"], "completed");

    let report: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true, "audit chain should verify: {report}");
}
