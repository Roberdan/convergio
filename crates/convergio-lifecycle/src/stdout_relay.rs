//! Relay sub-process stdout lines to the plan bus as `agent:<id>:stdout`.

use convergio_bus::{Bus, NewMessage};
use serde_json::json;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::ChildStdout;
use tracing::warn;

/// Read lines from `stdout` and publish each to the plan bus.
///
/// Topic is `agent:{process_id}:stdout`. Runs until stdout closes
/// (the subprocess exits). Fire-and-forget: spawn with
/// [`tokio::spawn`].
pub(crate) async fn relay(stdout: ChildStdout, plan_id: String, process_id: String, bus: Bus) {
    let topic = format!("agent:{}:stdout", process_id);
    let reader = BufReader::new(stdout);
    let mut lines = reader.lines();
    let mut seq: u64 = 0;
    loop {
        match lines.next_line().await {
            Ok(Some(line)) => {
                let msg = NewMessage {
                    plan_id: plan_id.clone(),
                    topic: topic.clone(),
                    sender: Some(process_id.clone()),
                    payload: json!({ "type": "stdout", "text": line, "seq": seq }),
                };
                if let Err(e) = bus.publish(msg).await {
                    warn!(process_id = %process_id, error = %e, "stdout relay: publish failed");
                }
                seq += 1;
            }
            Ok(None) => break,
            Err(e) => {
                // Read failures (closed pipe, decode error, ...) are
                // observability events: log them so an operator can
                // tell "relay ended because EOF" from "relay ended
                // because the pipe broke". The loop terminates either
                // way -- the child's stdout cannot be re-opened.
                warn!(
                    process_id = %process_id,
                    error = %e,
                    "stdout relay: read failed, terminating relay"
                );
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convergio_bus::Bus;
    use convergio_db::Pool;
    use tempfile::tempdir;
    use tokio::process::Command;

    async fn boot_pool() -> (Pool, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let url = format!("sqlite://{}/state.db", dir.path().display());
        let pool = Pool::connect(&url).await.unwrap();
        crate::init(&pool).await.unwrap();
        convergio_bus::init(&pool).await.unwrap();
        (pool, dir)
    }

    #[tokio::test]
    async fn relay_publishes_stdout_lines_to_bus() {
        let (pool, _dir) = boot_pool().await;
        let bus = Bus::new(pool.clone());
        let plan_id = "plan-relay-test".to_string();
        let process_id = "proc-1".to_string();

        // Spawn a process that prints two JSONL lines.
        let mut child = Command::new("sh")
            .args(["-c", r#"echo '{"a":1}'; echo '{"b":2}'"#])
            .stdout(std::process::Stdio::piped())
            .spawn()
            .expect("sh not available");
        let stdout = child.stdout.take().unwrap();

        relay(stdout, plan_id.clone(), process_id.clone(), bus.clone()).await;

        // Both lines must be on the bus.
        let msgs = bus
            .poll(&plan_id, &format!("agent:{}:stdout", process_id), 0, 10)
            .await
            .unwrap();
        assert_eq!(msgs.len(), 2, "expected 2 messages");
        let first = msgs[0]
            .payload
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        let second = msgs[1]
            .payload
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(
            first.contains("{\"a\":1}") || first.contains(r#"{"a":1}"#),
            "first: {first}"
        );
        assert!(
            second.contains("{\"b\":2}") || second.contains(r#"{"b":2}"#),
            "second: {second}"
        );
        assert_eq!(msgs[0].payload["seq"], 0);
        assert_eq!(msgs[1].payload["seq"], 1);
    }
}
