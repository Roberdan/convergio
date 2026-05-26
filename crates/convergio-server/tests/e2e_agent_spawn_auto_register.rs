//! E2E coverage for the `cvg agent spawn` auto-register + heartbeat
//! contract introduced by issue #176. Validates the wire shape that
//! `crates/convergio-cli/src/commands/agent_spawn_heartbeat.rs` posts.
//!
//! The unit tests in convergio-cli pin the JSON body shape; this file
//! exercises the round-trip against an in-process daemon so a
//! breaking change to the registry schema fails CI loudly.

#![allow(clippy::expect_used, clippy::unwrap_used)]

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
    let pool = Pool::connect(&format!("sqlite://{}", db_path.display()))
        .await
        .unwrap();
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

/// Full happy-path: register → heartbeat(working) → heartbeat(idle).
/// Mirrors the sequence `cvg agent spawn` performs around the vendor
/// CLI invocation.
#[tokio::test]
async fn spawn_register_heartbeat_idle_round_trip() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();
    let agent_id = "claude-sonnet-spawn-test";
    let task_id = "task-12345";

    // 1. Register — same body shape as build_register_body() in CLI.
    let registered: Value = client
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({
            "id": agent_id,
            "kind": "claude",
            "name": format!("{agent_id} (claude/sonnet)"),
            "host": "macOS-test",
            "capabilities": ["code", "test"],
            "metadata": {
                "spawned_by": "cvg agent spawn",
                "current_task_id": task_id,
            }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(registered["id"], agent_id);
    assert_eq!(registered["kind"], "claude");
    assert_eq!(registered["host"], "macOS-test");

    // 2. Heartbeat with current_task_id + status=working — what the
    //    background loop posts every HEARTBEAT_INTERVAL.
    let beat_working: Value = client
        .post(format!(
            "{base}/v1/agent-registry/agents/{agent_id}/heartbeat"
        ))
        .json(&json!({"current_task_id": task_id, "status": "working"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(beat_working["status"], "working");
    assert_eq!(beat_working["current_task_id"], task_id);

    // 3. Final heartbeat on clean exit — status=idle.
    let beat_idle: Value = client
        .post(format!(
            "{base}/v1/agent-registry/agents/{agent_id}/heartbeat"
        ))
        .json(&json!({"current_task_id": task_id, "status": "idle"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(beat_idle["status"], "idle");

    // 4. Verify the registry sees the agent with the latest status.
    let listed: Value = client
        .get(format!("{base}/v1/agent-registry/agents"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = listed.as_array().expect("list endpoint returns array");
    let me = arr
        .iter()
        .find(|a| a["id"] == agent_id)
        .expect("registered agent must appear in list");
    assert_eq!(me["status"], "idle");
}

/// Exit with `terminated` when the vendor CLI fails — the spawn flow
/// posts this final heartbeat in the error branch.
#[tokio::test]
async fn spawn_register_then_terminated_on_failure() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();
    let agent_id = "claude-sonnet-fail-test";

    client
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({
            "id": agent_id,
            "kind": "claude",
            "host": "h",
            "capabilities": [],
            "metadata": {"spawned_by": "cvg agent spawn"}
        }))
        .send()
        .await
        .unwrap();

    let beat: Value = client
        .post(format!(
            "{base}/v1/agent-registry/agents/{agent_id}/heartbeat"
        ))
        .json(&json!({"status": "terminated"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(beat["status"], "terminated");
}
