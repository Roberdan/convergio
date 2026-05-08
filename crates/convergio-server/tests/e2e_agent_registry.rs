//! Agent registry API E2E tests.

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
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
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
    let client = reqwest::Client::new();
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

#[tokio::test]
async fn usage_evidence_aggregates_into_agent_metadata() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let _agent: Value = client
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({
            "id": "agent-u",
            "kind": "claude",
            "name": "usage test",
            "host": "terminal",
            "capabilities": ["code"],
            "metadata": {}
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "usage plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "usage task", "evidence_required": []}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    let _task: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "agent-u"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let _evidence: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "usage",
            "payload": {
                "input_tokens": 11,
                "output_tokens": 22,
                "model": "claude-opus-4",
                "cost_usd": 0.0123
            }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let agent: Value = client
        .get(format!("{base}/v1/agent-registry/agents/agent-u"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(agent["metadata"]["usage"]["calls"], 1);
    assert_eq!(agent["metadata"]["usage"]["total_input_tokens"], 11);
    assert_eq!(agent["metadata"]["usage"]["total_output_tokens"], 22);
    assert_eq!(agent["metadata"]["usage"]["last_model"], "claude-opus-4");
}
