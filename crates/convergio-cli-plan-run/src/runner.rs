//! Wave-grouped plan runner with optional intra-wave concurrency (P1-8).

use crate::{Client, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;
use futures_util::stream::{FuturesUnordered, StreamExt};
use serde_json::{json, Value};

const MAX_PARALLEL_BOUNDS: std::ops::RangeInclusive<u8> = 1..=16;

#[derive(Clone, Debug)]
struct TaskMeta {
    id: String,
    title: String,
    wave: i64,
    sequence: i64,
}

/// Iterate pending tasks grouped by wave; within a wave, run up to
/// `max_parallel` claim+submit pairs concurrently. Halts on the first
/// failure and prints a localised resume hint.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    id: &str,
    agent_id: Option<&str>,
    max_parallel: u8,
) -> Result<()> {
    let max_parallel = max_parallel.clamp(*MAX_PARALLEL_BOUNDS.start(), *MAX_PARALLEL_BOUNDS.end());

    let plan: Value = match client.get::<Value>(&format!("/v1/plans/{id}")).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", bundle.t("plan-not-found", &[("id", id)]));
            return Err(e);
        }
    };
    let plan_id = sfield(&plan, "id", id).to_string();
    let plan_number = plan.get("number").and_then(Value::as_i64).unwrap_or(0);
    let plan_title = sfield(&plan, "title", "?").to_string();

    let tasks: Value = client.get(&format!("/v1/plans/{plan_id}/tasks")).await?;
    let pending = collect_pending_in_wave_order(&tasks);
    say(
        bundle,
        output,
        "plan-run-started",
        &[
            ("number", &plan_number.to_string()),
            ("title", &plan_title),
            ("pending", &pending.len().to_string()),
        ],
    );

    let mut completed = 0usize;
    for wave_tasks in group_by_wave(pending) {
        for (task, outcome) in run_wave(
            client,
            bundle,
            output,
            &plan_id,
            agent_id,
            max_parallel,
            wave_tasks,
        )
        .await
        {
            if let Err(err) = outcome {
                halt(bundle, output, &task, &err, plan_number);
                return Err(err);
            }
            completed += 1;
        }
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

fn halt(
    bundle: &Bundle,
    output: OutputMode,
    task: &TaskMeta,
    err: &anyhow::Error,
    plan_number: i64,
) {
    say(
        bundle,
        output,
        "plan-run-halted",
        &[
            ("wave", &task.wave.to_string()),
            ("seq", &task.sequence.to_string()),
            ("title", &task.title),
            ("error", &err.to_string()),
        ],
    );
    if plan_number > 0 {
        say(
            bundle,
            output,
            "plan-run-resume-hint",
            &[("number", &plan_number.to_string())],
        );
    }
}

fn collect_pending_in_wave_order(tasks: &Value) -> Vec<TaskMeta> {
    let mut out: Vec<TaskMeta> = tasks
        .as_array()
        .map(|a| {
            a.iter()
                .filter(|t| t.get("status").and_then(Value::as_str) == Some("pending"))
                .map(|t| TaskMeta {
                    id: sfield(t, "id", "?").to_string(),
                    title: sfield(t, "title", "?").to_string(),
                    wave: t.get("wave").and_then(Value::as_i64).unwrap_or(0),
                    sequence: t.get("sequence").and_then(Value::as_i64).unwrap_or(0),
                })
                .collect()
        })
        .unwrap_or_default();
    out.sort_by_key(|t| (t.wave, t.sequence));
    out
}

fn group_by_wave(pending: Vec<TaskMeta>) -> Vec<Vec<TaskMeta>> {
    let mut waves: Vec<Vec<TaskMeta>> = Vec::new();
    for t in pending {
        match waves.last_mut() {
            Some(w) if w.first().is_some_and(|h| h.wave == t.wave) => w.push(t),
            _ => waves.push(vec![t]),
        }
    }
    waves
}

fn sfield<'a>(v: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or(fallback)
}

fn say(bundle: &Bundle, output: OutputMode, key: &str, args: &[(&str, &str)]) {
    if matches!(output, OutputMode::Human) {
        println!("{}", bundle.t(key, args));
    }
}

async fn run_wave(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    plan_id: &str,
    agent_id: Option<&str>,
    max_parallel: u8,
    wave: Vec<TaskMeta>,
) -> Vec<(TaskMeta, Result<()>)> {
    let mut in_flight = FuturesUnordered::new();
    let mut iter = wave.into_iter();
    for _ in 0..max_parallel {
        match iter.next() {
            Some(t) => in_flight.push(submit(client, bundle, output, plan_id, agent_id, t)),
            None => break,
        }
    }
    let mut results = Vec::new();
    while let Some((meta, outcome)) = in_flight.next().await {
        let halt = outcome.is_err();
        results.push((meta, outcome));
        if halt {
            break;
        }
        if let Some(next) = iter.next() {
            in_flight.push(submit(client, bundle, output, plan_id, agent_id, next));
        }
    }
    results
}

async fn submit(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    plan_id: &str,
    agent_id: Option<&str>,
    task: TaskMeta,
) -> (TaskMeta, Result<()>) {
    let outcome = submit_one(client, plan_id, agent_id, &task).await;
    if outcome.is_ok() {
        say(
            bundle,
            output,
            "plan-run-task-submitted",
            &[
                ("wave", &task.wave.to_string()),
                ("seq", &task.sequence.to_string()),
                ("title", &task.title),
            ],
        );
    }
    (task, outcome)
}

async fn submit_one(
    client: &Client,
    plan_id: &str,
    agent_id: Option<&str>,
    t: &TaskMeta,
) -> Result<()> {
    let path = format!("/v1/tasks/{}/transition", t.id);
    client
        .post::<Value, Value>(&path, &transition_body(agent_id, "in_progress"))
        .await?;
    client
        .post::<Value, Value>(&path, &transition_body(agent_id, "submitted"))
        .await?;
    let _ = client
        .post::<Value, Value>(
            &format!("/v1/plans/{plan_id}/messages"),
            &json!({
                "topic": "plan.run",
                "payload": {
                    "event": "task.submitted",
                    "task_id": t.id,
                    "wave": t.wave,
                    "sequence": t.sequence,
                    "title": t.title,
                }
            }),
        )
        .await;
    Ok(())
}

fn transition_body(agent_id: Option<&str>, target: &str) -> Value {
    match agent_id {
        Some(a) => json!({ "target": target, "agent_id": a }),
        None => json!({ "target": target }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(wave: i64, seq: i64) -> TaskMeta {
        TaskMeta {
            id: format!("t{wave}{seq}"),
            title: "x".into(),
            wave,
            sequence: seq,
        }
    }

    #[test]
    fn group_by_wave_partitions_and_keeps_order() {
        let waves = group_by_wave(vec![meta(1, 1), meta(1, 2), meta(2, 1)]);
        assert_eq!(waves.len(), 2);
        assert_eq!(waves[0].len(), 2);
        assert_eq!(waves[1][0].sequence, 1);
    }

    #[test]
    fn collect_pending_filters_and_sorts() {
        let raw = json!([
            {"id":"a","title":"A","status":"done","wave":1,"sequence":1},
            {"id":"b","title":"B","status":"pending","wave":2,"sequence":1},
            {"id":"c","title":"C","status":"pending","wave":1,"sequence":2},
        ]);
        let ids: Vec<String> = collect_pending_in_wave_order(&raw)
            .into_iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(ids, ["c", "b"]);
    }

    #[test]
    fn transition_body_carries_agent_id_when_present() {
        assert!(transition_body(None, "submitted").get("agent_id").is_none());
        assert_eq!(transition_body(Some("a"), "in_progress")["agent_id"], "a");
    }
}
