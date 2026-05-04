//! HTTP end-to-end test for Layer 2 (`convergio-bus`).

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
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
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
async fn publish_poll_ack_round_trip_over_http() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();
    let plan_id = "plan-x";

    // Publish two messages.
    for i in 0..2 {
        let _: Value = client
            .post(format!("{base}/v1/plans/{plan_id}/messages"))
            .json(&json!({
                "topic": "task.done",
                "sender": "agent-1",
                "payload": {"i": i},
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    }

    // Poll.
    let messages: Vec<Value> = client
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages?topic=task.done&limit=10"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["payload"]["i"], 0);
    assert_eq!(messages[1]["payload"]["i"], 1);

    // Ack the first one.
    let id = messages[0]["id"].as_str().unwrap();
    let _: Value = client
        .post(format!("{base}/v1/messages/{id}/ack"))
        .json(&json!({"consumer": "agent-2"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // Poll again — only the second should remain.
    let messages: Vec<Value> = client
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages?topic=task.done&limit=10"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["payload"]["i"], 1);
}

#[tokio::test]
async fn ack_unknown_returns_404() {
    let (base, _dir) = boot().await;
    let resp = reqwest::Client::new()
        .post(format!("{base}/v1/messages/no-such-id/ack"))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "not_found");
}

#[tokio::test]
async fn corrupt_message_timestamp_returns_invalid_timestamp() {
    let (base, dir) = boot().await;
    let client = reqwest::Client::new();
    let plan_id = "plan-corrupt";
    let message: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/messages"))
        .json(&json!({"topic": "task.done", "payload": {}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = message["id"].as_str().unwrap();

    let pool = Pool::connect(&format!("sqlite://{}/state.db", dir.path().display()))
        .await
        .unwrap();
    sqlx::query("UPDATE agent_messages SET created_at = 'not-a-timestamp' WHERE id = ?")
        .bind(id)
        .execute(pool.inner())
        .await
        .unwrap();

    let resp = client
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages?topic=task.done&limit=10"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 500);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "invalid_timestamp");
}

#[tokio::test]
async fn poll_rejects_unbounded_limit() {
    let (base, _dir) = boot().await;
    let resp = reqwest::Client::new()
        .get(format!("{base}/v1/plans/plan-x/messages?topic=t&limit=101"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "invalid_message_limit");
}

#[tokio::test]
async fn tail_returns_acked_messages_and_topics_summarises() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();
    let plan_id = "plan-tail";

    for (topic, i) in [("alpha", 0), ("alpha", 1), ("beta", 0)] {
        let _: Value = client
            .post(format!("{base}/v1/plans/{plan_id}/messages"))
            .json(&json!({"topic": topic, "sender": "a", "payload": {"i": i}}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    }

    // Ack the first alpha message.
    let polled: Vec<Value> = client
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages?topic=alpha&limit=10"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let id = polled[0]["id"].as_str().unwrap();
    let _: Value = client
        .post(format!("{base}/v1/messages/{id}/ack"))
        .json(&json!({"consumer": "h"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // tail with no topic filter sees all 3 (consumed + unconsumed).
    let all: Vec<Value> = client
        .get(format!("{base}/v1/plans/{plan_id}/messages/tail?limit=10"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(all.len(), 3);

    // tail with topic filter sees only that topic, including the acked one.
    let alpha: Vec<Value> = client
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages/tail?topic=alpha&limit=10"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(alpha.len(), 2);
    assert!(alpha.iter().any(|m| m["consumed_at"].is_string()));

    // topics summary lists both topics with correct counts.
    let topics: Vec<Value> = client
        .get(format!("{base}/v1/plans/{plan_id}/topics"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(topics.len(), 2);
    let alpha_summary = topics.iter().find(|t| t["topic"] == "alpha").unwrap();
    let beta_summary = topics.iter().find(|t| t["topic"] == "beta").unwrap();
    assert_eq!(alpha_summary["count"], 2);
    assert_eq!(beta_summary["count"], 1);
}

#[tokio::test]
async fn tail_supports_since_cursor() {
    let (base, _dir) = boot().await;
    let client = reqwest::Client::new();
    let plan_id = "plan-cursor";
    for i in 0..4 {
        let _: Value = client
            .post(format!("{base}/v1/plans/{plan_id}/messages"))
            .json(&json!({"topic": "x", "payload": {"i": i}}))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    }
    let after_two: Vec<Value> = client
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages/tail?cursor=2&limit=10"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(after_two.len(), 2);
    assert_eq!(after_two[0]["payload"]["i"], 2);
    assert_eq!(after_two[1]["payload"]["i"], 3);
}
