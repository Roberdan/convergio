//! `cvg plan triage` — surface stale pending/failed tasks.

use super::{Client, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;
use serde_json::{json, Value};
use std::io::{self, Write};

/// Run `cvg plan triage`.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    id: &str,
    stale_days: i64,
    auto_close: bool,
) -> Result<()> {
    let path = format!("/v1/plans/{id}/triage?stale_days={stale_days}");
    let tasks: Value = client.get(&path).await?;
    let arr = tasks.as_array().cloned().unwrap_or_default();
    let count = arr.len() as i64;
    let days_str = stale_days.to_string();
    let count_str = count.to_string();
    match output {
        OutputMode::Human => {
            if count == 0 {
                println!("{}", bundle.t("plan-triage-empty", &[("days", &days_str)]));
            } else {
                println!(
                    "{}",
                    bundle.t_n_with("plan-triage-header", count, &[("days", &days_str)])
                );
                for task in &arr {
                    print_task_line(bundle, task);
                }
                if auto_close {
                    close_stale(client, bundle, id, &arr, &count_str, stale_days).await?;
                }
            }
        }
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&tasks)?);
        }
        OutputMode::Plain => {
            for task in &arr {
                if let Some(tid) = task.get("id").and_then(Value::as_str) {
                    println!("{tid}");
                }
            }
        }
    }
    Ok(())
}

fn print_task_line(bundle: &Bundle, task: &Value) {
    let tid = task["id"].as_str().unwrap_or("?");
    let title = task["title"].as_str().unwrap_or("?");
    let status = task["status"].as_str().unwrap_or("?");
    let wave = task["wave"].as_i64().unwrap_or(0).to_string();
    let seq = task["sequence"].as_i64().unwrap_or(0).to_string();
    let updated_at = task["updated_at"].as_str().unwrap_or("?");
    println!(
        "{}",
        bundle.t(
            "plan-triage-line",
            &[
                ("id", tid),
                ("title", title),
                ("status", status),
                ("wave", &wave),
                ("seq", &seq),
                ("updated_at", updated_at),
            ]
        )
    );
}

async fn close_stale(
    client: &Client,
    bundle: &Bundle,
    plan_id: &str,
    arr: &[Value],
    count_str: &str,
    stale_days: i64,
) -> Result<()> {
    eprint!(
        "{} ",
        bundle.t("plan-triage-confirm", &[("count", count_str)])
    );
    io::stderr().flush().ok();
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    let answer = line.trim().to_lowercase();
    if answer == "y" || answer == "s" {
        let mut closed = 0u32;
        for task in arr {
            let tid = task["id"].as_str().unwrap_or("");
            if tid.is_empty() {
                continue;
            }
            let reason =
                format!("auto-closed by triage (plan {plan_id}): stale for {stale_days} days",);
            client
                .post::<_, Value>(
                    &format!("/v1/tasks/{tid}/close-post-hoc"),
                    &json!({ "reason": reason }),
                )
                .await?;
            closed += 1;
        }
        println!(
            "{}",
            bundle.t("plan-triage-closed", &[("count", &closed.to_string())])
        );
    } else {
        println!("{}", bundle.t("plan-triage-skipped", &[]));
    }
    Ok(())
}
