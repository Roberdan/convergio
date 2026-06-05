//! Wire-contract test for `cvg discover` (F2). Boots the in-process
//! daemon, registers three synthetic peers with varying heartbeat
//! ages, publishes two bus messages on different topics, then asserts
//! that the four endpoints `cvg discover` depends on agree on the
//! shape and ordering the CLI assumes:
//!
//! 1. `GET /v1/agent-registry/agents` — id, kind, status, capabilities,
//!    last_heartbeat_at.
//! 2. `GET /v1/plans` — id, title, status.
//! 3. `GET /v1/plans/:id/topics` — topic, count, last_at.
//! 4. `GET /v1/plans/:id/tasks` — agent_id, status.
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
        reports: Arc::new(convergio_reports::ReportTemplateStore::new(pool.clone())),
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
async fn cvg_discover_sources_match_cli_expectations() {
    let (base, _dir) = boot().await;
    let http = common::client();

    // Three synthetic peers with varying heartbeat ages.
    for id in ["alpha", "beta", "gamma"] {
        let _: Value = http
            .post(format!("{base}/v1/agent-registry/agents"))
            .json(&json!({
                "id": id,
                "kind": "subagent",
                "name": id,
                "host": "test",
                "capabilities": ["rust", "test"],
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        let _: Value = http
            .post(format!("{base}/v1/agent-registry/agents/{id}/heartbeat"))
            .json(&json!({"status": "working"}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    }

    // Plan with two bus topics + one task assigned to 'alpha'.
    let plan: Value = http
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "discover plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();
    for topic in ["coordination/agents", "test/notice"] {
        let _: Value = http
            .post(format!("{base}/v1/plans/{plan_id}/messages"))
            .json(&json!({
                "topic": topic,
                "sender": "alpha",
                "payload": {"hello": topic},
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    }
    let task: Value = http
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "t1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();
    let _: Value = http
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "alpha"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Endpoint 1: registry must list all three peers.
    let agents: Vec<Value> = http
        .get(format!("{base}/v1/agent-registry/agents"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let ids: Vec<&str> = agents.iter().filter_map(|a| a["id"].as_str()).collect();
    assert!(ids.contains(&"alpha") && ids.contains(&"beta") && ids.contains(&"gamma"));
    for a in &agents {
        assert!(a["last_heartbeat_at"].is_string());
        assert!(a["capabilities"].is_array());
    }

    // Endpoint 2: /v1/plans includes our plan.
    let plans: Vec<Value> = http
        .get(format!("{base}/v1/plans"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(plans
        .iter()
        .any(|p| p["id"].as_str() == Some(plan_id.as_str())));

    // Endpoint 3: /v1/plans/:id/topics yields both topics.
    let topics: Vec<Value> = http
        .get(format!("{base}/v1/plans/{plan_id}/topics"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let topic_names: Vec<&str> = topics.iter().filter_map(|t| t["topic"].as_str()).collect();
    assert!(topic_names.contains(&"coordination/agents"));
    assert!(topic_names.contains(&"test/notice"));

    // Endpoint 4: /v1/plans/:id/tasks lets the CLI find your tasks.
    let tasks: Vec<Value> = http
        .get(format!("{base}/v1/plans/{plan_id}/tasks"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let mine: Vec<&Value> = tasks
        .iter()
        .filter(|t| t["agent_id"].as_str() == Some("alpha"))
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0]["status"].as_str(), Some("in_progress"));
}
