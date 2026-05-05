//! `cvg plan run` — sequential plan runner.

use super::{Client, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;
use serde_json::{json, Value};

/// Iterate pending tasks in wave/seq order: claim each, submit it, and
/// publish a bus announcement. Halts with a non-zero exit on the first error.
pub(super) async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    id: &str,
    agent_id: Option<&str>,
) -> Result<()> {
    let plan: Value = match client.get::<Value>(&format!("/v1/plans/{id}")).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", bundle.t("plan-not-found", &[("id", id)]));
            return Err(e);
        }
    };
    let plan_id = plan.get("id").and_then(Value::as_str).unwrap_or(id);
    let plan_number = plan.get("number").and_then(Value::as_i64).unwrap_or(0);
    let plan_title = plan.get("title").and_then(Value::as_str).unwrap_or("?");

    let tasks: Value = client.get(&format!("/v1/plans/{plan_id}/tasks")).await?;
    let mut pending: Vec<&Value> = tasks
        .as_array()
        .map(|a| {
            let mut v: Vec<&Value> = a
                .iter()
                .filter(|t| t.get("status").and_then(Value::as_str) == Some("pending"))
                .collect();
            v.sort_by_key(|t| {
                (
                    t.get("wave").and_then(Value::as_i64).unwrap_or(0),
                    t.get("sequence").and_then(Value::as_i64).unwrap_or(0),
                )
            });
            v
        })
        .unwrap_or_default();
    let pending_owned: Vec<Value> = pending.drain(..).cloned().collect();

    if matches!(output, OutputMode::Human) {
        println!(
            "{}",
            bundle.t(
                "plan-run-started",
                &[
                    ("number", &plan_number.to_string()),
                    ("title", plan_title),
                    ("pending", &pending_owned.len().to_string()),
                ]
            )
        );
    }

    let mut completed = 0usize;
    for task in &pending_owned {
        let task_id = task.get("id").and_then(Value::as_str).unwrap_or("?");
        let wave = task.get("wave").and_then(Value::as_i64).unwrap_or(0);
        let seq = task.get("sequence").and_then(Value::as_i64).unwrap_or(0);
        let title = task.get("title").and_then(Value::as_str).unwrap_or("?");

        let claim_body = if let Some(aid) = agent_id {
            json!({ "target": "in_progress", "agent_id": aid })
        } else {
            json!({ "target": "in_progress" })
        };
        if let Err(e) = client
            .post::<Value, Value>(&format!("/v1/tasks/{task_id}/transition"), &claim_body)
            .await
        {
            emit_halt(bundle, output, wave, seq, title, &e.to_string());
            return Err(e);
        }

        let submit_body = if let Some(aid) = agent_id {
            json!({ "target": "submitted", "agent_id": aid })
        } else {
            json!({ "target": "submitted" })
        };
        if let Err(e) = client
            .post::<Value, Value>(&format!("/v1/tasks/{task_id}/transition"), &submit_body)
            .await
        {
            emit_halt(bundle, output, wave, seq, title, &e.to_string());
            return Err(e);
        }

        // Bus failure is non-fatal.
        let _ = client
            .post::<Value, Value>(
                &format!("/v1/plans/{plan_id}/messages"),
                &json!({
                    "topic": "plan.run",
                    "payload": {
                        "event": "task.submitted",
                        "task_id": task_id,
                        "wave": wave,
                        "sequence": seq,
                        "title": title,
                    }
                }),
            )
            .await;

        if matches!(output, OutputMode::Human) {
            println!(
                "{}",
                bundle.t(
                    "plan-run-task-submitted",
                    &[
                        ("wave", &wave.to_string()),
                        ("seq", &seq.to_string()),
                        ("title", title),
                    ]
                )
            );
        }
        completed += 1;
    }

    if matches!(output, OutputMode::Human) {
        println!(
            "{}",
            bundle.t(
                "plan-run-complete",
                &[
                    ("number", &plan_number.to_string()),
                    ("count", &completed.to_string()),
                ]
            )
        );
    } else if matches!(output, OutputMode::Plain) {
        println!("{completed}");
    }
    Ok(())
}

fn emit_halt(bundle: &Bundle, output: OutputMode, wave: i64, seq: i64, title: &str, error: &str) {
    if matches!(output, OutputMode::Human) {
        println!(
            "{}",
            bundle.t(
                "plan-run-halted",
                &[
                    ("wave", &wave.to_string()),
                    ("seq", &seq.to_string()),
                    ("title", title),
                    ("error", error),
                ]
            )
        );
    }
}
