//! HTTP helpers for [`crate::plan_execution`].
//!
//! Each function does exactly one daemon call. Keeps `plan_execution.rs`
//! orchestration-only and under the 300-line cap.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Deserialize;

/// Minimal task shape returned by `GET /v1/plans/:plan_id/tasks`.
#[derive(Debug, Deserialize)]
pub(crate) struct Task {
    pub id: String,
    pub title: String,
    pub status: String,
}

/// Minimal evidence shape returned by `GET /v1/tasks/:id/evidence`.
#[derive(Debug, Deserialize)]
pub(crate) struct EvidenceItem {
    #[allow(dead_code)]
    pub id: String,
    pub kind: String,
    #[allow(dead_code)]
    pub created_at: DateTime<Utc>,
}

/// Minimal agent shape returned by `GET /v1/agent-registry/agents`.
#[derive(Debug, Deserialize)]
pub(crate) struct AgentEntry {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub status: String,
}

/// Minimal bus message shape returned by `GET /v1/plans/:plan_id/messages`.
#[derive(Debug, Deserialize)]
pub(crate) struct BusMessage {
    #[allow(dead_code)]
    pub seq: i64,
    pub sender: String,
    pub topic: String,
}

/// Fetch all tasks for a plan.
pub(crate) async fn fetch_tasks(
    client: &reqwest::Client,
    daemon: &str,
    plan_id: &str,
) -> Result<Vec<Task>> {
    let url = format!(
        "{}/v1/plans/{}/tasks",
        daemon.trim_end_matches('/'),
        plan_id
    );
    client
        .get(&url)
        .send()
        .await
        .context("fetch tasks")?
        .json()
        .await
        .context("decode tasks")
}

/// Fetch evidence for a task. Returns empty vec on any error (advisory).
pub(crate) async fn fetch_evidence(
    client: &reqwest::Client,
    daemon: &str,
    task_id: &str,
) -> Vec<EvidenceItem> {
    let url = format!(
        "{}/v1/tasks/{}/evidence",
        daemon.trim_end_matches('/'),
        task_id
    );
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
        _ => vec![],
    }
}

/// Fetch registry agents. Returns empty vec on any error (advisory).
pub(crate) async fn fetch_agents(client: &reqwest::Client, daemon: &str) -> Vec<AgentEntry> {
    let url = format!("{}/v1/agent-registry/agents", daemon.trim_end_matches('/'));
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
        _ => vec![],
    }
}

/// Fetch the latest bus messages for a plan. Returns empty vec on any error.
///
/// The endpoint returns NDJSON (one JSON object per line), not a JSON array.
pub(crate) async fn fetch_bus_messages(
    client: &reqwest::Client,
    daemon: &str,
    plan_id: &str,
) -> Vec<BusMessage> {
    let url = format!(
        "{}/v1/plans/{}/messages?limit=200",
        daemon.trim_end_matches('/'),
        plan_id
    );
    match client.get(&url).send().await {
        Ok(r) if r.status().is_success() => {
            let text = r.text().await.unwrap_or_default();
            text.lines()
                .filter(|l| !l.is_empty())
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect()
        }
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Json, Router};
    use tokio::net::TcpListener;

    async fn spawn(router: Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        format!("http://{addr}")
    }

    // Regression test for audit finding `plan_execution_scan.rs:108`:
    // `GET /v1/plans/:plan_id/messages` returns a JSON array, not NDJSON,
    // so per-line parsing always yields an empty vec and `bus_ok` is
    // permanently false. With the fix this returns one decoded message.
    #[tokio::test]
    async fn fetch_bus_messages_decodes_json_array() {
        let router = Router::new().route(
            "/v1/plans/:plan_id/messages",
            get(|| async {
                Json(serde_json::json!([
                    {
                        "id": "msg-1",
                        "seq": 1,
                        "plan_id": "plan-1",
                        "topic": "task.done",
                        "sender": "agent-a",
                        "payload": {},
                        "consumed_at": null,
                        "consumed_by": null,
                        "created_at": "2026-01-01T00:00:00Z"
                    }
                ]))
            }),
        );
        let base = spawn(router).await;
        let client = reqwest::Client::new();
        let msgs = fetch_bus_messages(&client, &base, "plan-1").await;
        assert_eq!(msgs.len(), 1, "expected 1 message decoded from JSON array");
        assert_eq!(msgs[0].sender, "agent-a");
        assert_eq!(msgs[0].topic, "task.done");
    }
}
