//! Auto-register + heartbeat for `cvg agent spawn` (issue #176).
//!
//! Before the vendor CLI is exec-ed, the spawned agent is registered
//! in `/v1/agent-registry/agents` so `cvg coherence agents` and
//! `cvg dash` can see it. While the runner is alive, a background
//! tokio task posts heartbeats every 60 s. On exit a final heartbeat
//! flips status to `idle` (clean) or `terminated` (failure).
//!
//! All HTTP calls are best-effort: a daemon hiccup must not abort the
//! vendor CLI. Errors are logged with `eprintln!` and execution
//! continues — the vendor CLI's success is the source of truth, not
//! the registry.

use super::Client;
use convergio_runner::RunnerKind;
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

/// How often the heartbeat fires while the runner is alive. Matches
/// the reaper's default 60s window so a missed beat is detected
/// before the lifecycle watcher times the agent out.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(60);

/// Build the JSON body for `POST /v1/agent-registry/agents`.
///
/// Public so the unit test can pin the wire shape.
pub fn build_register_body(
    agent_id: &str,
    kind: &RunnerKind,
    host: &str,
    task_id: &str,
) -> serde_json::Value {
    json!({
        "id": agent_id,
        "kind": kind.vendor,
        "name": format!("{} ({}/{})", agent_id, kind.vendor, kind.model),
        "host": host,
        "capabilities": ["code", "test"],
        "metadata": {
            "spawned_by": "cvg agent spawn",
            "current_task_id": task_id,
        }
    })
}

/// Build the JSON body for `POST /v1/agent-registry/agents/:id/heartbeat`.
pub fn build_heartbeat_body(task_id: Option<&str>, status: &str) -> serde_json::Value {
    let mut body = json!({"status": status});
    if let Some(tid) = task_id {
        body["current_task_id"] = json!(tid);
    }
    body
}

/// Resolve a host label for the registry. Tries `$HOSTNAME` then
/// `hostname(1)` semantics; falls back to `"unknown"`.
pub fn resolve_host() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| {
            std::process::Command::new("hostname")
                .output()
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "unknown".to_owned())
        })
}

/// Best-effort POST to register the agent. Logs and swallows errors
/// (a daemon hiccup must not abort the vendor CLI).
pub async fn register_agent(client: &Client, agent_id: &str, kind: &RunnerKind, task_id: &str) {
    let body = build_register_body(agent_id, kind, &resolve_host(), task_id);
    let res: Result<serde_json::Value, _> = client.post("/v1/agent-registry/agents", &body).await;
    if let Err(e) = res {
        eprintln!("warning: agent registry register failed: {e}");
    }
}

/// Best-effort heartbeat POST.
pub async fn post_heartbeat(client: &Client, agent_id: &str, task_id: Option<&str>, status: &str) {
    let body = build_heartbeat_body(task_id, status);
    let path = format!("/v1/agent-registry/agents/{agent_id}/heartbeat");
    let res: Result<serde_json::Value, _> = client.post(&path, &body).await;
    if let Err(e) = res {
        eprintln!("warning: agent heartbeat failed: {e}");
    }
}

/// Owns the background heartbeat task; aborts on drop and flushes a
/// final heartbeat via [`Self::finish`].
pub struct HeartbeatGuard {
    handle: JoinHandle<()>,
    stop: Arc<Notify>,
    client: Client,
    agent_id: String,
    task_id: String,
}

impl HeartbeatGuard {
    /// Start the heartbeat loop on the current tokio runtime.
    pub fn start(client: Client, agent_id: String, task_id: String) -> Self {
        let stop = Arc::new(Notify::new());
        let stop_in = stop.clone();
        let client_in = client.clone();
        let agent_in = agent_id.clone();
        let task_in = task_id.clone();
        let handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(HEARTBEAT_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            interval.tick().await; // skip immediate fire (registration just ran)
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        post_heartbeat(&client_in, &agent_in, Some(&task_in), "working").await;
                    }
                    () = stop_in.notified() => break,
                }
            }
        });
        Self {
            handle,
            stop,
            client,
            agent_id,
            task_id,
        }
    }

    /// Stop the loop and post one final heartbeat with the given status
    /// (`"idle"` for clean exit, `"terminated"` for failure).
    pub async fn finish(self, final_status: &str) {
        self.stop.notify_one();
        let _ = self.handle.await;
        post_heartbeat(
            &self.client,
            &self.agent_id,
            Some(&self.task_id),
            final_status,
        )
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_body_has_stable_shape() {
        let kind = RunnerKind::claude_sonnet();
        let body = build_register_body("claude-sonnet-abc1234", &kind, "macOS-test", "task-7777");
        assert_eq!(body["id"], "claude-sonnet-abc1234");
        assert_eq!(body["kind"], "claude");
        assert_eq!(body["host"], "macOS-test");
        assert_eq!(body["metadata"]["spawned_by"], "cvg agent spawn");
        assert_eq!(body["metadata"]["current_task_id"], "task-7777");
        assert!(body["capabilities"].is_array());
    }

    #[test]
    fn heartbeat_body_includes_task_when_present() {
        let body = build_heartbeat_body(Some("t1"), "working");
        assert_eq!(body["status"], "working");
        assert_eq!(body["current_task_id"], "t1");
    }

    #[test]
    fn heartbeat_body_omits_task_when_absent() {
        let body = build_heartbeat_body(None, "idle");
        assert_eq!(body["status"], "idle");
        assert!(body.get("current_task_id").is_none());
    }

    #[test]
    fn resolve_host_returns_non_empty() {
        let h = resolve_host();
        assert!(
            !h.is_empty(),
            "host should never be empty (falls back to 'unknown')"
        );
    }

    #[test]
    fn register_body_uses_kind_vendor_and_model() {
        let kind = RunnerKind::copilot_gpt();
        let body = build_register_body("copilot-gpt-5.2-zzz", &kind, "h", "t");
        let name = body["name"].as_str().unwrap();
        assert!(name.contains("copilot"), "name should mention vendor");
        assert!(name.contains("gpt"), "name should mention model");
    }
}
