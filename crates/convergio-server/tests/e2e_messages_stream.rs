//! P1.1 — `/v1/plans/:plan_id/messages/stream` SSE end-to-end test.

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
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

async fn read_sse_events(
    mut body: reqwest::Response,
    event_kind: &str,
    n: usize,
    timeout: Duration,
) -> Vec<Value> {
    let mut buf: Vec<u8> = Vec::new();
    let mut events: Vec<Value> = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;

    while events.len() < n {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let chunk = tokio::time::timeout(remaining, body.chunk()).await;
        let bytes = match chunk {
            Ok(Ok(Some(b))) => b,
            _ => break,
        };
        buf.extend_from_slice(&bytes);
        while let Some(end) = buf.windows(2).position(|w| w == b"\n\n") {
            let frame = String::from_utf8_lossy(&buf[..end]).to_string();
            buf.drain(..end + 2);
            let mut current_kind: Option<String> = None;
            let mut data_payload: Option<String> = None;
            for line in frame.lines() {
                if let Some(stripped) = line.strip_prefix("event: ") {
                    current_kind = Some(stripped.to_string());
                } else if let Some(stripped) = line.strip_prefix("data: ") {
                    data_payload = Some(stripped.to_string());
                }
            }
            if current_kind.as_deref() == Some(event_kind) {
                if let Some(d) = data_payload {
                    if let Ok(v) = serde_json::from_str::<Value>(&d) {
                        events.push(v);
                    }
                }
            }
        }
    }
    events
}

#[tokio::test]
async fn messages_stream_emits_published_messages() {
    let (base, _dir) = boot().await;
    let plan_id = "plan-stream-x";

    // Open the SSE stream first; `since=0` so we see history+future.
    let stream_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();
    let resp = stream_client
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages/stream?topic=task.done&since=0"
        ))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    assert_eq!(
        resp.headers()
            .get("content-type")
            .map(|v| v.to_str().unwrap_or_default()),
        Some("text/event-stream")
    );

    // Publish three messages on the matching topic.
    let pub_client = reqwest::Client::new();
    for i in 0..3 {
        let _: Value = pub_client
            .post(format!("{base}/v1/plans/{plan_id}/messages"))
            .json(&json!({
                "topic": "task.done",
                "sender": "agent-A",
                "payload": {"i": i},
            }))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
    }

    let events = read_sse_events(resp, "bus", 3, Duration::from_secs(8)).await;
    assert_eq!(events.len(), 3, "expected 3 bus events, got {events:?}");
    let mut prev = 0_i64;
    for (idx, ev) in events.iter().enumerate() {
        let seq = ev["seq"].as_i64().expect("seq present");
        assert!(seq > prev, "non-monotonic seq");
        prev = seq;
        assert_eq!(ev["topic"], "task.done");
        assert_eq!(ev["sender"], "agent-A");
        assert_eq!(ev["plan_id"], plan_id);
        assert_eq!(ev["payload"]["i"], idx as i64);
        assert!(ev["created_at"].is_string());
    }
}

#[tokio::test]
async fn messages_stream_filters_by_topic() {
    let (base, _dir) = boot().await;
    let plan_id = "plan-filter-y";
    let stream_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();
    let resp = stream_client
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages/stream?topic=coordination/agents&since=0"
        ))
        .send()
        .await
        .unwrap();

    let pub_client = reqwest::Client::new();
    // Publish on the wrong topic first — must NOT be delivered.
    let _: Value = pub_client
        .post(format!("{base}/v1/plans/{plan_id}/messages"))
        .json(&json!({"topic": "task.done", "payload": {"x": 1}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    // Then publish the matching one.
    let _: Value = pub_client
        .post(format!("{base}/v1/plans/{plan_id}/messages"))
        .json(&json!({"topic": "coordination/agents", "payload": {"x": 2}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let events = read_sse_events(resp, "bus", 1, Duration::from_secs(5)).await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["topic"], "coordination/agents");
    assert_eq!(events[0]["payload"]["x"], 2);
}
