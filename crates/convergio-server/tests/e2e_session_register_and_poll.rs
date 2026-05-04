//! Integration coverage for `cvg session register-and-poll`.
//!
//! Exercises the same HTTP wire the CLI calls — register, heartbeat,
//! list active plans, poll `agent:<id>` and `plan:<id>` topics — and
//! asserts the new `/v1/status.telemetry` block reflects the freshly
//! registered session. The CLI smoke test in
//! `crates/convergio-cli-session/tests/cli_smoke_session_register.rs`
//! covers the clap surface; this file covers the daemon contract.

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
async fn register_heartbeat_and_poll_round_trip_is_audited() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();
    let agent_id = "claude-code-test";

    // 1. Register.
    let agent: Value = client
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({
            "id": agent_id,
            "kind": "claude",
            "host": "macbook",
            "capabilities": ["code", "test", "doc"],
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(agent["id"], agent_id);

    // 2. Heartbeat.
    let beat: Value = client
        .post(format!(
            "{base}/v1/agent-registry/agents/{agent_id}/heartbeat"
        ))
        .json(&json!({"status": "idle"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(beat["status"], "idle");

    // 3. List plans (empty).
    let plans: Value = client
        .get(format!("{base}/v1/plans"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(plans.as_array().unwrap().is_empty());

    // 4. Polling against a freshly-created plan returns an empty
    //    inbox without error — the failure mode parallel sessions
    //    hit on 2026-05-04 (silently skipping poll because no plan).
    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "Coordination plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();
    let inbox: Value = client
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages?topic=agent:{agent_id}&limit=20"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(inbox.as_array().unwrap().is_empty());

    // 5. Telemetry block surfaces the registration AND the
    //    audit-emitted `agent.session_started` row.
    let status: Value = client
        .get(format!("{base}/v1/status"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let t = &status["telemetry"];
    assert_eq!(t["agents_registered_total"], 1);
    assert_eq!(t["agents_active_24h"], 1);
    assert_eq!(t["sessions_started_24h"], 1);
    assert_eq!(t["plans_active"], 1);
    assert!(t["audit_rows_total"].as_i64().unwrap() >= 3);
    assert_eq!(t["bus_messages_24h"], 0);
    assert_eq!(t["workspace_leases_active"], 0);

    // Audit chain remains intact.
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
