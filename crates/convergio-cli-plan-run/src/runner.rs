//! Wave-grouped plan runner with optional intra-wave concurrency (P1-8).
//!
//! Public entry point only. Wave/submit machinery lives in [`crate::wave`]
//! to keep this file under the per-file Rust line cap (AGENTS.md).

use crate::wave::{
    collect_pending_in_wave_order, group_by_wave, run_wave, sfield, SubmitOutcome, TaskMeta,
};
use crate::{Client, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;
use serde_json::Value;

const MAX_PARALLEL_BOUNDS: std::ops::RangeInclusive<u8> = 1..=16;

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
        for outcome in run_wave(
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
            let SubmitOutcome {
                task,
                transition,
                bus_warning,
            } = outcome;
            if let Some(err) = bus_warning {
                // P5: localized, non-fatal warning so swallowed publish
                // failures are at least observable to the operator.
                eprintln!(
                    "{}",
                    bundle.t(
                        "plan-run-bus-warning",
                        &[
                            ("wave", &task.wave.to_string()),
                            ("seq", &task.sequence.to_string()),
                            ("title", &task.title),
                            ("error", &err.to_string()),
                        ]
                    )
                );
            }
            if let Err(err) = transition {
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
    if is_missing_evidence_refusal(&err.to_string()) {
        say(
            bundle,
            output,
            "plan-run-missing-evidence-hint",
            &[("task_id", &task.id)],
        );
    }
    if plan_number > 0 {
        say(
            bundle,
            output,
            "plan-run-resume-hint",
            &[("number", &plan_number.to_string())],
        );
    }
}

/// Detect the daemon's `gate_refused` + `missing_evidence_kind` pair on
/// the wire. The server formats refusals as `HTTP 409 ... gate_refused ...
/// missing_evidence_kind: <list>` (see `convergio-server::error` and
/// `convergio-durability::gates::evidence_gate`), so we match on both
/// substrings to avoid false positives from unrelated `gate_refused`
/// errors that point at a different fix.
fn is_missing_evidence_refusal(msg: &str) -> bool {
    msg.contains("gate_refused") && msg.contains("missing_evidence_kind")
}

pub(crate) fn say(bundle: &Bundle, output: OutputMode, key: &str, args: &[(&str, &str)]) {
    if matches!(output, OutputMode::Human) {
        println!("{}", bundle.t(key, args));
    }
}

#[cfg(test)]
mod tests {
    use super::is_missing_evidence_refusal;

    #[test]
    fn detects_missing_evidence_refusal() {
        let msg = r#"HTTP 409 Conflict: {"code":"gate_refused","message":"evidence: missing_evidence_kind: tests"}"#;
        assert!(is_missing_evidence_refusal(msg));
    }

    #[test]
    fn ignores_unrelated_gate_refusal() {
        let msg =
            r#"HTTP 409 Conflict: {"code":"gate_refused","message":"no_debt: lingering warnings"}"#;
        assert!(!is_missing_evidence_refusal(msg));
    }

    #[test]
    fn ignores_non_gate_errors() {
        assert!(!is_missing_evidence_refusal("HTTP 500: boom"));
    }
}
