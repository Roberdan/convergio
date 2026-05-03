//! HTTP end-to-end test for Layer 3 (`convergio-lifecycle`).

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
async fn spawn_get_heartbeat_round_trip_over_http() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let proc: Value = client
        .post(format!("{base}/v1/agents/spawn"))
        .json(&json!({
            "kind": "shell",
            "command": "/bin/echo",
            "args": ["e2e"],
            "env": [],
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = proc["id"].as_str().unwrap();
    assert_eq!(proc["kind"], "shell");
    assert_eq!(proc["status"], "running");
    assert!(proc["pid"].is_number());

    // GET should return the same row.
    let fetched: Value = client
        .get(format!("{base}/v1/agents/{id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched["id"], id);

    // Heartbeat ok.
    let hb: Value = client
        .post(format!("{base}/v1/agents/{id}/heartbeat"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(hb["ok"], true);
}

#[tokio::test]
async fn spawn_invalid_command_returns_422() {
    let (base, _dir) = boot().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/v1/agents/spawn"))
        .json(&json!({
            "kind": "shell",
            "command": "/no/such/binary/anywhere",
            "args": [],
            "env": [],
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "spawn_failed");
}

#[tokio::test]
async fn spawn_runner_registers_agent_claims_task_and_tracks_process() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "runner plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();
    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "runner task", "evidence_required": []}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    let spawned: Value = client
        .post(format!("{base}/v1/agents/spawn-runner"))
        .json(&json!({
            "agent_id": "shell-runner-01",
            "kind": "shell",
            "command": "/bin/sleep",
            "args": ["1"],
            "plan_id": plan_id,
            "task_id": task_id,
            "capabilities": ["shell"],
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(spawned["agent"]["id"], "shell-runner-01");
    assert_eq!(spawned["process"]["kind"], "shell");
    assert_eq!(spawned["task"]["status"], "in_progress");
    assert_eq!(spawned["task"]["agent_id"], "shell-runner-01");

    let agents: Vec<Value> = client
        .get(format!("{base}/v1/agent-registry/agents"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(agents.iter().any(|agent| agent["id"] == "shell-runner-01"));
}

#[tokio::test]
async fn spawn_runner_accepts_claude_kind() {
    // ADR-0028: kind=claude is a label for the same supervisor path,
    // routed by the operator to ~/.convergio/adapters/claude/run.sh.
    // The daemon does not require the wrapper to exist — `command`
    // may point anywhere local — so we use /bin/sleep here and just
    // verify the registry + process shape is correct.
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "claude runner plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();
    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "claude task", "evidence_required": []}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    let spawned: Value = client
        .post(format!("{base}/v1/agents/spawn-runner"))
        .json(&json!({
            "agent_id": "claude-runner-01",
            "kind": "claude",
            "command": "/bin/sleep",
            "args": ["1"],
            "plan_id": plan_id,
            "task_id": task_id,
            "capabilities": ["code"],
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(spawned["agent"]["id"], "claude-runner-01");
    assert_eq!(spawned["agent"]["kind"], "claude");
    assert_eq!(
        spawned["agent"]["metadata"]["runner"],
        "claude-shell-wrapper"
    );
    assert_eq!(spawned["process"]["kind"], "claude");
    assert_eq!(spawned["task"]["status"], "in_progress");
}

#[tokio::test]
async fn spawn_runner_rejects_unknown_kind() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/v1/agents/spawn-runner"))
        .json(&json!({
            "agent_id": "typo-runner-01",
            "kind": "cluade", // intentional typo
            "command": "/bin/sleep",
            "args": ["1"],
            "capabilities": [],
        }))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_client_error(),
        "expected 4xx for unknown kind, got {}",
        resp.status()
    );
    let body: Value = resp.json().await.unwrap();
    let msg = body["error"]["message"].as_str().unwrap_or("");
    assert!(
        msg.contains("unknown runner kind"),
        "error message did not mention unknown kind: {body}"
    );
}
