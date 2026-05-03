//! System-message route smoke tests (`/v1/system-messages`, ADR-0025).

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
async fn system_message_publish_then_poll_round_trip() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let published: Value = client
        .post(format!("{base}/v1/system-messages"))
        .json(&json!({
            "topic": "system.session-events",
            "sender": "claude-code-test-1",
            "payload": {
                "agent_id": "claude-code-test-1",
                "kind": "agent.attached"
            }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(published["topic"], "system.session-events");
    assert!(
        published["plan_id"].is_null(),
        "system messages must have plan_id IS NULL"
    );

    let polled: Vec<Value> = client
        .get(format!(
            "{base}/v1/system-messages?topic=system.session-events&limit=10"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(polled.len(), 1);
    assert_eq!(polled[0]["sender"], "claude-code-test-1");
    assert_eq!(polled[0]["payload"]["kind"], "agent.attached");
}

#[tokio::test]
async fn system_message_rejects_non_system_topic() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/v1/system-messages"))
        .json(&json!({
            "topic": "task.touched-file",
            "payload": {}
        }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error() || resp.status().is_server_error(),
        "non-system topic must be refused, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn system_message_rejects_invalid_limit() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();

    let resp = client
        .get(format!(
            "{base}/v1/system-messages?topic=system.session-events&limit=999"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "invalid_message_limit");
}
