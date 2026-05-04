//! `cvg coherence handshake` — 2-session multi-agent E2E smoke test.
//!
//! Exercises the full leash round-trip in one shot: create a synthetic
//! plan, register two synthetic agents, run a `ping`/`pong` exchange
//! on `coordination/handshake`, ack both messages, retire both agents
//! — all within the configured timeout.
//!
//! Failure modes are named: if phase 3 times out the report says
//! "B never saw A's ping", not "something went wrong". This is the
//! 5th verifier per ADR-0040 follow-up F1; sister verifiers cover
//! ADRs, routes, registry/PR alignment, and frontmatter drift.
//!
//! Pure HTTP client; no in-process daemon. Caller (CI / dev shell)
//! is expected to point `--daemon` at a running `convergio-server`.
//! Internationalised through `convergio-i18n` (P5).

use crate::handshake_http::{build_client, create_plan};
use crate::handshake_render::{render_human, render_plain};
use crate::handshake_run::run_phases;
use crate::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;
use serde::Serialize;
use serde_json::Value;
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Per-phase status of the handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseOutcome {
    /// Phase completed within timeout.
    Ok,
    /// Phase exceeded its deadline.
    Timeout,
    /// Phase failed for a non-timeout reason.
    Failed,
    /// Phase was not reached because an earlier phase failed.
    Skipped,
}

/// One phase of the 6-phase handshake.
#[derive(Debug, Clone, Serialize)]
pub struct Phase {
    /// Phase number (1..=6).
    pub n: u8,
    /// Short label: `register`, `ping`, `pong`, `receive`, `acks`, `retire`.
    pub label: String,
    /// Outcome bucket.
    pub outcome: PhaseOutcome,
    /// Wall-clock duration in ms (or until failure).
    pub elapsed_ms: u128,
    /// Free-form detail rendered in human/plain output.
    pub detail: String,
}

/// Aggregate result of one handshake run.
#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// Daemon base URL the run targeted.
    pub daemon: String,
    /// Configured timeout per phase, in ms.
    pub timeout_ms: u128,
    /// `true` if every phase came back `Ok`.
    pub success: bool,
    /// Total wall-clock duration of the entire run.
    pub total_elapsed_ms: u128,
    /// Per-phase breakdown.
    pub phases: Vec<Phase>,
    /// Synthetic plan id created for this run (kept as evidence).
    pub plan_id: String,
    /// Synthetic agent ids `(A, B)`.
    pub agent_ids: (String, String),
}

/// Run the handshake against `daemon` with `timeout_seconds` per phase.
///
/// Always returns `Ok(Report)`; the boolean `success` flag and
/// per-phase outcomes carry the verdict. The CLI shim turns
/// `success=false` into a non-zero exit.
pub async fn run_check(daemon: &str, timeout_seconds: u64) -> Result<Report> {
    let timeout = Duration::from_secs(timeout_seconds.max(1));
    let topic = "coordination/handshake".to_string();
    let nonce = Uuid::new_v4().simple().to_string()[..16].to_string();
    let agent_a = format!("handshake-A-{}", short_id());
    let agent_b = format!("handshake-B-{}", short_id());
    let started = Instant::now();
    let client = build_client(timeout)?;
    let mut phases: Vec<Phase> = Vec::with_capacity(6);

    let plan_id = match create_plan(&client, daemon).await {
        Ok(id) => id,
        Err(e) => {
            phases.push(phase_helper(
                0,
                "bootstrap",
                PhaseOutcome::Failed,
                started,
                &format!("plan create failed: {e}"),
            ));
            skip_remaining(&mut phases, 1);
            return Ok(finalize(
                daemon,
                timeout,
                started,
                phases,
                String::new(),
                (agent_a, agent_b),
            ));
        }
    };

    if let Err(failed) = run_phases(
        &client,
        daemon,
        &plan_id,
        &topic,
        &agent_a,
        &agent_b,
        &nonce,
        timeout,
        &mut phases,
    )
    .await
    {
        skip_remaining(&mut phases, failed);
    }

    Ok(finalize(
        daemon,
        timeout,
        started,
        phases,
        plan_id,
        (agent_a, agent_b),
    ))
}

fn finalize(
    daemon: &str,
    timeout: Duration,
    started: Instant,
    phases: Vec<Phase>,
    plan_id: String,
    agents: (String, String),
) -> Report {
    let success = phases.len() == 6 && phases.iter().all(|p| p.outcome == PhaseOutcome::Ok);
    Report {
        daemon: daemon.to_string(),
        timeout_ms: timeout.as_millis(),
        success,
        total_elapsed_ms: started.elapsed().as_millis(),
        phases,
        plan_id,
        agent_ids: agents,
    }
}

/// Render + run from the CLI; sets exit code 0/1 on success.
pub async fn run(
    bundle: &Bundle,
    output: OutputMode,
    daemon: &str,
    timeout_seconds: u64,
) -> Result<()> {
    let report = run_check(daemon, timeout_seconds).await?;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => render_plain(&report),
        OutputMode::Human => render_human(&report, bundle),
    }
    tracing::info!(
        target: "convergio.coherence.handshake",
        daemon = %report.daemon,
        success = report.success,
        total_elapsed_ms = report.total_elapsed_ms as u64,
        "coherence.handshake.run",
    );
    if !report.success {
        std::process::exit(1);
    }
    Ok(())
}

fn short_id() -> String {
    Uuid::new_v4().simple().to_string()[..8].to_string()
}

/// Build a [`Phase`] record. Helper used by both this module and
/// [`crate::handshake_run`].
pub(crate) fn phase_helper(
    n: u8,
    label: &str,
    outcome: PhaseOutcome,
    started: Instant,
    detail: &str,
) -> Phase {
    Phase {
        n,
        label: label.into(),
        outcome,
        elapsed_ms: started.elapsed().as_millis(),
        detail: detail.into(),
    }
}

/// Pad `phases` from `start_at` through phase 6 with `Skipped` entries.
pub(crate) fn skip_remaining(phases: &mut Vec<Phase>, start_at: u8) {
    for n in start_at..=6u8 {
        phases.push(Phase {
            n,
            label: phase_label(n).into(),
            outcome: PhaseOutcome::Skipped,
            elapsed_ms: 0,
            detail: String::new(),
        });
    }
}

fn phase_label(n: u8) -> &'static str {
    match n {
        1 => "register",
        2 => "ping",
        3 => "pong",
        4 => "receive",
        5 => "acks",
        6 => "retire",
        _ => "?",
    }
}

/// Validate that a candidate pong payload matches what we sent.
pub(crate) fn validate_pong_payload(payload: &Value, nonce: &str, replying_to: &str) -> bool {
    payload.get("type").and_then(Value::as_str) == Some("pong")
        && payload.get("nonce").and_then(Value::as_str) == Some(nonce)
        && payload.get("replying_to").and_then(Value::as_str) == Some(replying_to)
}

/// Test entry point for finalize used by [`crate::handshake_tests`].
#[cfg(test)]
pub(crate) fn test_finalize(
    daemon: &str,
    timeout: Duration,
    started: Instant,
    phases: Vec<Phase>,
    plan_id: String,
    agents: (String, String),
) -> Report {
    finalize(daemon, timeout, started, phases, plan_id, agents)
}

/// Test accessor for [`short_id`].
#[cfg(test)]
pub(crate) fn test_short_id() -> String {
    short_id()
}

/// Test accessor for [`phase_label`].
#[cfg(test)]
pub(crate) fn test_phase_label(n: u8) -> &'static str {
    phase_label(n)
}
