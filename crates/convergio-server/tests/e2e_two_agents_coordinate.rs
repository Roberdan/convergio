//! E2E test for Wave 0b PRD-001 KR1: two ephemeral agents must
//! register and become visible to each other through the daemon
//! within the heartbeat window.

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

async fn register(client: &reqwest::Client, base: &str, id: &str, host: &str) -> Value {
    client
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({
            "id": id,
            "kind": "claude",
            "name": format!("session-{id}"),
            "host": host,
            "capabilities": ["edit", "read", "shell", "evidence-attach"],
            "metadata": {
                "tty": "ttys000",
                "pid": 4242,
                "cwd": "/repo",
                "session_started_at": "2026-05-01T00:00:00Z"
            }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

async fn announce(client: &reqwest::Client, base: &str, agent_id: &str) {
    let resp = client
        .post(format!("{base}/v1/system-messages"))
        .json(&json!({
            "topic": "system.session-events",
            "sender": agent_id,
            "payload": {
                "agent_id": agent_id,
                "kind": "agent.attached"
            }
        }))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "publish failed: {}",
        resp.status()
    );
}

#[tokio::test]
async fn two_ephemeral_agents_register_and_see_each_other() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    // Two sibling sessions register concurrently.
    let alpha = register(&client, &base, "claude-code-alpha", "host-a").await;
    let beta = register(&client, &base, "claude-code-beta", "host-b").await;
    assert_eq!(alpha["id"], "claude-code-alpha");
    assert_eq!(beta["id"], "claude-code-beta");

    // Both publish presence on the system-scoped bus topic.
    announce(&client, &base, "claude-code-alpha").await;
    announce(&client, &base, "claude-code-beta").await;

    // The registry must list both — this is what `cvg status --agents`
    // surfaces to the operator.
    let listed: Vec<Value> = client
        .get(format!("{base}/v1/agent-registry/agents"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let ids: Vec<&str> = listed
        .iter()
        .map(|a| a["id"].as_str().unwrap_or(""))
        .collect();
    assert!(
        ids.contains(&"claude-code-alpha"),
        "alpha missing from registry: {ids:?}"
    );
    assert!(
        ids.contains(&"claude-code-beta"),
        "beta missing from registry: {ids:?}"
    );

    // Both presence messages landed on system.session-events with
    // `plan_id IS NULL` — peer sessions can poll this stream to see
    // the others.
    let messages: Vec<Value> = client
        .get(format!(
            "{base}/v1/system-messages?topic=system.session-events&limit=10"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(messages.len(), 2);
    for m in &messages {
        assert!(
            m["plan_id"].is_null(),
            "system message must have plan_id IS NULL"
        );
        assert_eq!(m["topic"], "system.session-events");
        assert_eq!(m["payload"]["kind"], "agent.attached");
    }
    let senders: Vec<&str> = messages
        .iter()
        .map(|m| m["sender"].as_str().unwrap_or(""))
        .collect();
    assert!(senders.contains(&"claude-code-alpha"));
    assert!(senders.contains(&"claude-code-beta"));
}

#[tokio::test]
async fn one_agent_retiring_does_not_affect_the_other() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    register(&client, &base, "claude-code-alpha", "host-a").await;
    register(&client, &base, "claude-code-beta", "host-b").await;

    // Alpha retires explicitly (Stop hook flow).
    client
        .post(format!(
            "{base}/v1/agent-registry/agents/claude-code-alpha/retire"
        ))
        .send()
        .await
        .unwrap();

    // Beta is still in the registry and still working.
    let beta: Value = client
        .get(format!("{base}/v1/agent-registry/agents/claude-code-beta"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(beta["id"], "claude-code-beta");
    assert_ne!(
        beta["status"], "terminated",
        "beta must keep its status when alpha retires"
    );

    // Alpha's record persists but is marked retired.
    let alpha: Value = client
        .get(format!("{base}/v1/agent-registry/agents/claude-code-alpha"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(alpha["status"], "retired");
}
