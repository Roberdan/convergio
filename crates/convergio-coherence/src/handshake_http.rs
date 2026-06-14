//! HTTP client helpers for [`crate::handshake`].
//!
//! Pure transport: each function does exactly one daemon call. Keeps
//! `handshake.rs` orchestration-only and under the 300-line cap.

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::{Duration, Instant};

// Mirrors convergio_api::PURPOSE_ID_HEADER; inlined to avoid a new crate dep.
const PURPOSE_ID_HEADER: &str = "x-purpose-id";
const HANDSHAKE_PURPOSE_ID: &str = "00000000-0000-4000-8000-000000000443";

/// Lite shape of a published bus message — only the fields the
/// handshake needs. Avoids depending on `convergio-bus`.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct BusMessage {
    /// Stable message id (used for acks + `replying_to`).
    pub id: String,
    /// Per-plan monotonic sequence.
    pub seq: i64,
    /// Caller-supplied JSON payload.
    pub payload: Value,
}

/// Phase failure cause — either a deadline exhaustion or any other error.
pub(crate) enum PhaseFail {
    /// Phase deadline elapsed before the expected event arrived.
    Timeout(String),
    /// Anything else (HTTP error, decode error, mismatched payload).
    Other(String),
}

/// Build an HTTP client with the per-phase timeout and default purpose-binding
/// header pre-applied. Callers that need a per-request purpose override (e.g.
/// `HANDSHAKE_PURPOSE_ID`) can still set the header on individual requests —
/// the per-request header takes precedence over the default.
pub(crate) fn build_client(timeout: Duration) -> Result<reqwest::Client> {
    // Default purpose id so the purpose-binding middleware accepts calls from
    // all daemon-backed verifiers. Handshake requests override per-request.
    let purpose = std::env::var("CONVERGIO_PURPOSE_ID")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| "00000000-0000-0000-0000-000000000000".to_string());
    let mut headers = reqwest::header::HeaderMap::new();
    if let Ok(v) = reqwest::header::HeaderValue::from_str(&purpose) {
        headers.insert(PURPOSE_ID_HEADER, v);
    }
    reqwest::Client::builder()
        .timeout(timeout)
        .default_headers(headers)
        .build()
        .with_context(|| "build http client")
}

/// `POST /v1/plans` — create a synthetic plan and return its id.
pub(crate) async fn create_plan(client: &reqwest::Client, daemon: &str) -> Result<String> {
    let title = format!(
        "coherence-handshake-{}",
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ")
    );
    let url = format!("{}/v1/plans", daemon.trim_end_matches('/'));
    let resp = client
        .post(&url)
        .header(PURPOSE_ID_HEADER, HANDSHAKE_PURPOSE_ID)
        .json(&json!({"title": title, "description": null, "project": "coherence"}))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("plan create returned {}", resp.status()));
    }
    let v: Value = resp.json().await.with_context(|| "decode plan")?;
    v.get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("plan response missing id"))
}

/// `POST /v1/agent-registry/agents` — register one synthetic agent.
pub(crate) async fn register_one(client: &reqwest::Client, daemon: &str, id: &str) -> Result<()> {
    let url = format!("{}/v1/agent-registry/agents", daemon.trim_end_matches('/'));
    let body = json!({
        "id": id,
        "kind": "synthetic",
        "name": format!("Coherence handshake {id}"),
        "host": "coherence-handshake",
        "capabilities": ["handshake"],
    });
    let resp = client
        .post(&url)
        .header(PURPOSE_ID_HEADER, HANDSHAKE_PURPOSE_ID)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("register {id} returned {}", resp.status()));
    }
    Ok(())
}

/// Register both `a` and `b`. Fails fast on either error.
pub(crate) async fn register_pair(
    client: &reqwest::Client,
    daemon: &str,
    a: &str,
    b: &str,
) -> Result<()> {
    register_one(client, daemon, a).await?;
    register_one(client, daemon, b).await?;
    Ok(())
}

/// `POST /v1/agent-registry/agents/:id/heartbeat` — heartbeat one
/// synthetic agent. Exercises the heartbeat seam so a regression in
/// the heartbeat route surfaces here instead of producing a
/// false-green E2E (Codex review on PR #197).
pub(crate) async fn heartbeat_one(client: &reqwest::Client, daemon: &str, id: &str) -> Result<()> {
    let url = format!(
        "{}/v1/agent-registry/agents/{}/heartbeat",
        daemon.trim_end_matches('/'),
        id
    );
    let resp = client
        .post(&url)
        .header(PURPOSE_ID_HEADER, HANDSHAKE_PURPOSE_ID)
        .json(&json!({"status": "working"}))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("heartbeat {id} returned {}", resp.status()));
    }
    Ok(())
}

/// Heartbeat both `a` and `b`. Fails fast on either error.
pub(crate) async fn heartbeat_pair(
    client: &reqwest::Client,
    daemon: &str,
    a: &str,
    b: &str,
) -> Result<()> {
    heartbeat_one(client, daemon, a).await?;
    heartbeat_one(client, daemon, b).await?;
    Ok(())
}

/// `POST /v1/plans/:plan_id/messages` — publish on the handshake topic.
pub(crate) async fn publish(
    client: &reqwest::Client,
    daemon: &str,
    plan_id: &str,
    topic: &str,
    sender: &str,
    payload: &Value,
) -> Result<BusMessage> {
    let url = format!(
        "{}/v1/plans/{}/messages",
        daemon.trim_end_matches('/'),
        plan_id
    );
    let resp = client
        .post(&url)
        .header(PURPOSE_ID_HEADER, HANDSHAKE_PURPOSE_ID)
        .json(&json!({"topic": topic, "sender": sender, "payload": payload}))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("publish returned {}", resp.status()));
    }
    let m: BusMessage = resp.json().await.with_context(|| "decode message")?;
    Ok(m)
}

/// Poll a topic with `cursor=after_seq, exclude_sender=consumer` until
/// a message with `seq > after_seq` arrives or `timeout` expires.
pub(crate) async fn poll_for_seq(
    client: &reqwest::Client,
    daemon: &str,
    plan_id: &str,
    topic: &str,
    after_seq: i64,
    consumer: &str,
    timeout: Duration,
) -> Result<BusMessage, PhaseFail> {
    let deadline = Instant::now() + timeout;
    let url = format!(
        "{}/v1/plans/{}/messages",
        daemon.trim_end_matches('/'),
        plan_id
    );
    loop {
        if Instant::now() >= deadline {
            return Err(PhaseFail::Timeout(format!(
                "{consumer} never saw seq>{after_seq} on {topic} after {}ms",
                timeout.as_millis()
            )));
        }
        let resp = client
            .get(&url)
            .header(PURPOSE_ID_HEADER, HANDSHAKE_PURPOSE_ID)
            .query(&[
                ("topic", topic),
                ("cursor", &after_seq.to_string()),
                ("limit", "10"),
                ("exclude_sender", consumer),
            ])
            .send()
            .await
            .map_err(|e| PhaseFail::Other(format!("GET {url}: {e}")))?;
        if !resp.status().is_success() {
            return Err(PhaseFail::Other(format!("poll returned {}", resp.status())));
        }
        let msgs: Vec<BusMessage> = resp
            .json()
            .await
            .map_err(|e| PhaseFail::Other(format!("decode: {e}")))?;
        if let Some(m) = msgs.into_iter().find(|m| m.seq > after_seq) {
            return Ok(m);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// `POST /v1/messages/:id/ack` — ack one message as `consumer`.
pub(crate) async fn ack_one(
    client: &reqwest::Client,
    daemon: &str,
    message_id: &str,
    consumer: &str,
) -> Result<()> {
    let url = format!(
        "{}/v1/messages/{}/ack",
        daemon.trim_end_matches('/'),
        message_id
    );
    let resp = client
        .post(&url)
        .header(PURPOSE_ID_HEADER, HANDSHAKE_PURPOSE_ID)
        .json(&json!({"consumer": consumer}))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("ack returned {}", resp.status()));
    }
    Ok(())
}

/// Ack the pong from A and the ping from B.
pub(crate) async fn ack_pair(
    client: &reqwest::Client,
    daemon: &str,
    pong_id: &str,
    a: &str,
    ping_id: &str,
    b: &str,
) -> Result<()> {
    ack_one(client, daemon, pong_id, a).await?;
    ack_one(client, daemon, ping_id, b).await?;
    Ok(())
}

/// `POST /v1/agent-registry/agents/:id/retire` for one agent.
pub(crate) async fn retire_one(client: &reqwest::Client, daemon: &str, id: &str) -> Result<()> {
    let url = format!(
        "{}/v1/agent-registry/agents/{}/retire",
        daemon.trim_end_matches('/'),
        id
    );
    let resp = client
        .post(&url)
        .header(PURPOSE_ID_HEADER, HANDSHAKE_PURPOSE_ID)
        .json(&json!({}))
        .send()
        .await
        .with_context(|| format!("POST {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("retire {id} returned {}", resp.status()));
    }
    Ok(())
}

/// Retire both agents.
pub(crate) async fn retire_pair(
    client: &reqwest::Client,
    daemon: &str,
    a: &str,
    b: &str,
) -> Result<()> {
    retire_one(client, daemon, a).await?;
    retire_one(client, daemon, b).await?;
    Ok(())
}
