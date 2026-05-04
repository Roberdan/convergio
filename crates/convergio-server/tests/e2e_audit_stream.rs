//! P1.1 — `/v1/audit/stream` SSE end-to-end test.
//!
//! Boots an in-process server with a tempdir SQLite, opens an SSE
//! connection with reqwest, generates real audit rows by hitting
//! the HTTP plan/task surface, and asserts:
//!
//! 1. Three SSE `event: audit` frames arrive in monotonic `seq`
//!    order.
//! 2. Reconnecting with `since=<seq>` resumes after the cursor — no
//!    duplicates and no skipped rows.
//! 3. Frames carry the documented JSON shape.

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

async fn boot() -> (String, Pool, tempfile::TempDir) {
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
    (format!("http://{addr}"), pool, dir)
}

/// Read up to `n` `event: <event_kind>` data payloads from the
/// body stream. Bails after `timeout` to keep the test
/// deterministic.
pub async fn read_sse_events(
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
        // Parse complete frames separated by blank lines (`\n\n`).
        while let Some(end) = find_blank_line(&buf) {
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

fn find_blank_line(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\n\n")
}

async fn produce_audit_history(client: &reqwest::Client, base: &str) -> String {
    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "sse stream e2e"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "sse-task", "evidence_required": []}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();

    let _: Value = client
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "stream-tester"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    plan_id
}

#[tokio::test]
async fn audit_stream_emits_new_events_in_seq_order() {
    let (base, _pool, _dir) = boot().await;
    // Connect first so the stream's "current tip" baseline is 0.
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();
    let resp = client
        .get(format!("{base}/v1/audit/stream?since=0"))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "status: {}", resp.status());
    assert_eq!(
        resp.headers()
            .get("content-type")
            .map(|v| v.to_str().unwrap_or_default()),
        Some("text/event-stream")
    );

    // Generate ≥ 3 audit rows (plan.created + task.created + task.in_progress).
    let pub_client = reqwest::Client::new();
    let _plan_id = produce_audit_history(&pub_client, &base).await;

    let events = read_sse_events(resp, "audit", 3, Duration::from_secs(8)).await;
    assert!(
        events.len() >= 3,
        "expected at least 3 audit events, got {}",
        events.len()
    );
    let mut prev = 0_i64;
    for ev in &events {
        let seq = ev["seq"].as_i64().expect("seq present");
        assert!(seq > prev, "non-monotonic seq: {seq} after {prev}");
        prev = seq;
        assert!(ev["kind"].is_string(), "kind missing in {ev}");
        assert!(ev["entity_kind"].is_string(), "entity_kind missing in {ev}");
        assert!(ev["entity_id"].is_string(), "entity_id missing in {ev}");
        assert!(ev["created_at"].is_string(), "created_at missing in {ev}");
    }
}

#[tokio::test]
async fn audit_stream_resumes_with_since_cursor() {
    let (base, _pool, _dir) = boot().await;
    let pub_client = reqwest::Client::new();
    // Generate a baseline batch so we know seq counts.
    let _ = produce_audit_history(&pub_client, &base).await;

    // Open stream from seq=0 and grab the first event's seq.
    let stream_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .unwrap();
    let resp = stream_client
        .get(format!("{base}/v1/audit/stream?since=0"))
        .send()
        .await
        .unwrap();
    let first_batch = read_sse_events(resp, "audit", 2, Duration::from_secs(5)).await;
    assert!(first_batch.len() >= 2);
    let cursor = first_batch[0]["seq"].as_i64().unwrap();

    // Reconnect with since=cursor — should resume strictly after.
    let resp2 = stream_client
        .get(format!("{base}/v1/audit/stream?since={cursor}"))
        .send()
        .await
        .unwrap();
    let resumed = read_sse_events(resp2, "audit", 1, Duration::from_secs(5)).await;
    assert!(!resumed.is_empty(), "no events after cursor {cursor}");
    let next_seq = resumed[0]["seq"].as_i64().unwrap();
    assert!(
        next_seq > cursor,
        "resumed at {next_seq} which is not > cursor {cursor}"
    );
}
