//! `cvg agent heartbeat` — post a single heartbeat to the registry.
//!
//! Thin wrapper around `POST /v1/agent-registry/agents/:id/heartbeat`.
//! Intended for use in Claude Code `PostToolUse` hooks where a
//! background heartbeat beat loop is not available.

use super::{Client, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;
use serde_json::{json, Value};

/// Arguments for `cvg agent heartbeat`.
pub struct Args {
    /// Stable agent id.
    pub agent_id: String,
    /// Status string sent to the registry (e.g. `working`, `idle`).
    pub status: String,
}

/// Entry point.
pub async fn run(client: &Client, bundle: &Bundle, output: OutputMode, args: Args) -> Result<()> {
    let body = json!({"status": args.status});
    let path = format!("/v1/agent-registry/agents/{}/heartbeat", args.agent_id);
    let result: Value = client.post(&path, &body).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputMode::Plain => {
            let status = result.get("status").and_then(Value::as_str).unwrap_or("ok");
            println!("heartbeat\t{}\t{}", args.agent_id, status);
        }
        OutputMode::Human => {
            println!(
                "{}",
                bundle.t(
                    "agent-heartbeat-ok",
                    &[("id", &args.agent_id), ("status", &args.status)],
                )
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn args_fields_are_accessible() {
        let a = Args {
            agent_id: "claude-code-alice".to_string(),
            status: "working".to_string(),
        };
        assert_eq!(a.agent_id, "claude-code-alice");
        assert_eq!(a.status, "working");
    }
}
