//! Wave orchestration for `cvg plan run`: claim+submit one wave of
//! tasks at a time with bounded intra-wave concurrency. Split from
//! `runner.rs` to honour the per-file 300-line cap.

use crate::{Client, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;
use futures_util::stream::{FuturesUnordered, StreamExt};
use serde_json::{json, Value};
use std::future::Future;

#[derive(Clone, Debug)]
pub(crate) struct TaskMeta {
    pub id: String,
    pub title: String,
    pub wave: i64,
    pub sequence: i64,
}

/// One submitted task's full outcome: the transition result plus an
/// optional non-fatal warning from the plan-bus publish step.
pub(crate) struct SubmitOutcome {
    pub task: TaskMeta,
    pub transition: Result<()>,
    pub bus_warning: Option<anyhow::Error>,
}

pub(crate) async fn run_wave(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    plan_id: &str,
    agent_id: Option<&str>,
    max_parallel: u8,
    wave: Vec<TaskMeta>,
) -> Vec<SubmitOutcome> {
    run_wave_with(max_parallel, wave, |task| {
        submit(client, bundle, output, plan_id, agent_id, task)
    })
    .await
}

/// Generic wave orchestrator. Schedules up to `max_parallel` submit
/// futures at a time over `wave`, using `submit_fn` to produce each
/// future. Separated from the HTTP-driven path so the orchestration
/// semantics can be unit-tested without a daemon.
pub(crate) async fn run_wave_with<F, Fut>(
    max_parallel: u8,
    wave: Vec<TaskMeta>,
    submit_fn: F,
) -> Vec<SubmitOutcome>
where
    F: Fn(TaskMeta) -> Fut,
    Fut: Future<Output = SubmitOutcome>,
{
    let mut in_flight = FuturesUnordered::new();
    let mut iter = wave.into_iter();
    for _ in 0..max_parallel {
        match iter.next() {
            Some(t) => in_flight.push(submit_fn(t)),
            None => break,
        }
    }
    let mut results = Vec::new();
    let mut failed = false;
    while let Some(outcome) = in_flight.next().await {
        if outcome.transition.is_err() {
            failed = true;
        }
        results.push(outcome);
        if failed {
            // Stop scheduling new submissions, but drain the futures
            // already in flight so already-claimed tasks are not
            // stranded in `in_progress` on the daemon.
            continue;
        }
        if let Some(next) = iter.next() {
            in_flight.push(submit_fn(next));
        }
    }
    results
}

pub(crate) async fn submit(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    plan_id: &str,
    agent_id: Option<&str>,
    task: TaskMeta,
) -> SubmitOutcome {
    let (transition, bus_warning) = submit_one(client, plan_id, agent_id, &task).await;
    if transition.is_ok() {
        crate::runner::say(
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
    SubmitOutcome {
        task,
        transition,
        bus_warning,
    }
}

async fn submit_one(
    client: &Client,
    plan_id: &str,
    agent_id: Option<&str>,
    t: &TaskMeta,
) -> (Result<()>, Option<anyhow::Error>) {
    let path = format!("/v1/tasks/{}/transition", t.id);
    let claim_body = transition_body(agent_id, "in_progress");
    let submit_body = transition_body(agent_id, "submitted");
    let publish_path = format!("/v1/plans/{plan_id}/messages");
    let publish_body = json!({
        "topic": "plan.run",
        "payload": {
            "event": "task.submitted",
            "task_id": t.id,
            "wave": t.wave,
            "sequence": t.sequence,
            "title": t.title,
        }
    });
    submit_one_inner(
        || client.post::<Value, Value>(&path, &claim_body),
        || client.post::<Value, Value>(&path, &submit_body),
        || client.post::<Value, Value>(&publish_path, &publish_body),
    )
    .await
}

/// Pure orchestration of one task's transition pipeline. Splits the
/// three HTTP calls behind closures so the publish-error policy can
/// be unit-tested without a live daemon.
async fn submit_one_inner<C, S, P, Cf, Sf, Pf>(
    claim_fn: C,
    submit_fn: S,
    publish_fn: P,
) -> (Result<()>, Option<anyhow::Error>)
where
    C: FnOnce() -> Cf,
    S: FnOnce() -> Sf,
    P: FnOnce() -> Pf,
    Cf: Future<Output = Result<Value>>,
    Sf: Future<Output = Result<Value>>,
    Pf: Future<Output = Result<Value>>,
{
    if let Err(e) = claim_fn().await {
        return (Err(e), None);
    }
    if let Err(e) = submit_fn().await {
        return (Err(e), None);
    }
    let bus_warning = publish_fn().await.err();
    (Ok(()), bus_warning)
}

pub(crate) fn transition_body(agent_id: Option<&str>, target: &str) -> Value {
    match agent_id {
        Some(a) => json!({ "target": target, "agent_id": a }),
        None => json!({ "target": target }),
    }
}

pub(crate) fn collect_pending_in_wave_order(tasks: &Value) -> Vec<TaskMeta> {
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

pub(crate) fn group_by_wave(pending: Vec<TaskMeta>) -> Vec<Vec<TaskMeta>> {
    let mut waves: Vec<Vec<TaskMeta>> = Vec::new();
    for t in pending {
        match waves.last_mut() {
            Some(w) if w.first().is_some_and(|h| h.wave == t.wave) => w.push(t),
            _ => waves.push(vec![t]),
        }
    }
    waves
}

pub(crate) fn sfield<'a>(v: &'a Value, key: &str, fallback: &'a str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or(fallback)
}

#[cfg(test)]
#[path = "wave_tests.rs"]
mod tests;
