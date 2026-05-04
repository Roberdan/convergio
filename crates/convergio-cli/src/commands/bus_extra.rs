//! `cvg bus topics` and `cvg bus post` — secondary bus verbs.
//!
//! Split out of [`super::bus`] so the streaming-oriented dispatcher
//! file stays under the 300-line cap. The two verbs share nothing
//! except the [`super::bus::Plan`] resolver, which is re-exported
//! `pub(super)` in `bus.rs`.

use anyhow::{Context, Result};
use serde_json::Value;

use super::bus::resolve_plan;
use super::{Client, OutputMode};

/// `cvg bus topics`: per-topic summaries on a plan.
pub async fn topics(
    client: &Client,
    output: OutputMode,
    plan_id: Option<&str>,
    project: &str,
) -> Result<()> {
    let plan = resolve_plan(client, plan_id, project).await?;
    let summaries: Vec<Value> = client.get(&format!("/v1/plans/{}/topics", plan.id)).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&summaries)?),
        OutputMode::Plain => {
            for s in &summaries {
                let t = s.get("topic").and_then(Value::as_str).unwrap_or("?");
                let c = s.get("count").and_then(Value::as_i64).unwrap_or(0);
                let last = s.get("last_seq").and_then(Value::as_i64).unwrap_or(0);
                println!("topic={t} count={c} last_seq={last}");
            }
        }
        OutputMode::Human => {
            println!("Plan {} ({} topics)", plan.id, summaries.len());
            for s in &summaries {
                let t = s.get("topic").and_then(Value::as_str).unwrap_or("?");
                let c = s.get("count").and_then(Value::as_i64).unwrap_or(0);
                let last = s.get("last_seq").and_then(Value::as_i64).unwrap_or(0);
                let at = s.get("last_at").and_then(Value::as_str).unwrap_or("?");
                println!("  - {t} ({c} msgs, last seq={last} at {at})");
            }
        }
    }
    Ok(())
}

/// `cvg bus post`: publish a JSON payload to a topic.
pub async fn post(
    client: &Client,
    output: OutputMode,
    plan_id: Option<&str>,
    project: &str,
    topic: &str,
    payload: &str,
    sender: Option<&str>,
) -> Result<()> {
    let plan = resolve_plan(client, plan_id, project).await?;
    let payload: Value = serde_json::from_str(payload)
        .with_context(|| format!("payload must be valid JSON: {payload}"))?;
    let body = serde_json::json!({
        "topic": topic,
        "payload": payload,
        "sender": sender,
    });
    let m: Value = client
        .post(&format!("/v1/plans/{}/messages", plan.id), &body)
        .await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&m)?),
        OutputMode::Plain => {
            let seq = m.get("seq").and_then(Value::as_i64).unwrap_or(0);
            let id = m.get("id").and_then(Value::as_str).unwrap_or("?");
            println!("seq={seq} id={id}");
        }
        OutputMode::Human => {
            let seq = m.get("seq").and_then(Value::as_i64).unwrap_or(0);
            println!("Posted to {topic} on plan {} as seq {seq}.", plan.id);
        }
    }
    Ok(())
}
