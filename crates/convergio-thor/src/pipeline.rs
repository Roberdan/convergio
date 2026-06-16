//! Smart Thor pipeline execution (T3.02, ADR-0052).
//!
//! Thor still treats the pipeline command as trusted-local
//! configuration (operator-owned, never copied from plans, evidence,
//! agent output or HTTP requests). What changed in T3.02:
//!
//! 1. A built-in recipe sentinel — `cargo:auto` — runs `cargo fmt
//!    --check`, `clippy -D warnings`, and `cargo test --workspace` as
//!    separate steps, followed by optional coverage/a11y/i18n checks
//!    (W3). Each step gets its own truncated tail so a failing clippy
//!    doesn't drown a separately failing test.
//! 2. Timeout is configurable via
//!    `CONVERGIO_THOR_PIPELINE_TIMEOUT_SECS` (still defaults to 600s).
//! 3. Every run — pass or fail — appends a `pipeline.run` audit row
//!    so verifiable history exists even when Thor passes silently.
//! 4. Skip-when-trusted: if every task in the validated set already
//!    carries a `pipeline_run` evidence row whose `worktree_rev`
//!    matches the current working tree revision
//!    (`CONVERGIO_THOR_WORKTREE_REV`), the pipeline is treated as
//!    pre-validated and Thor returns `Pass` immediately. This is the
//!    seam fleet runs use to avoid re-running cargo on every repo.

use crate::steps::{run_step, StepOutcome, CARGO_AUTO_STEPS};
use convergio_durability::{audit::EntityKind, Durability};
use serde::Serialize;
use std::{process::Stdio, time::Duration};
use tokio::{process::Command, time::timeout};

pub(crate) const PIPELINE_ENV: &str = "CONVERGIO_THOR_PIPELINE_CMD";
pub(crate) const PIPELINE_TIMEOUT_ENV: &str = "CONVERGIO_THOR_PIPELINE_TIMEOUT_SECS";
pub(crate) const WORKTREE_REV_ENV: &str = "CONVERGIO_THOR_WORKTREE_REV";
pub(crate) const DEFAULT_PIPELINE_TIMEOUT_SECS: u64 = 600;
pub(crate) const CARGO_AUTO_SENTINEL: &str = "cargo:auto";

const PIPELINE_TAIL_BYTES: usize = 4096;

/// Normalized Thor pipeline configuration.
#[derive(Clone)]
pub(crate) struct Config {
    recipe: Recipe,
    timeout: Duration,
}

#[derive(Clone)]
enum Recipe {
    /// One-shot shell command (back-compat for operators who already
    /// configured `CONVERGIO_THOR_PIPELINE_CMD=make ci`).
    Shell(String),
    /// Built-in multi-step cargo recipe.
    CargoAuto,
}

/// Structured outcome of one pipeline run (canonicalized into the
/// `pipeline.run` audit row).
#[derive(Serialize)]
pub(crate) struct RunReport {
    recipe: &'static str,
    ok: bool,
    failing_step: Option<String>,
    duration_ms: u128,
    steps_attempted: usize,
    /// Number of optional steps that were skipped because the required
    /// binary was absent or no matching test targets were found (W3).
    steps_skipped: usize,
}

pub(crate) fn default_timeout() -> Duration {
    let secs = std::env::var(PIPELINE_TIMEOUT_ENV)
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_PIPELINE_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

pub(crate) fn from_env() -> Option<Config> {
    from_command(std::env::var(PIPELINE_ENV).ok(), default_timeout())
}

pub(crate) fn from_command(command: Option<String>, timeout: Duration) -> Option<Config> {
    command
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let recipe = if s == CARGO_AUTO_SENTINEL {
                Recipe::CargoAuto
            } else {
                Recipe::Shell(s)
            };
            Config { recipe, timeout }
        })
}

/// Run the configured pipeline, emit a `pipeline.run` audit row, and
/// return a verdict reason on failure.
pub(crate) async fn run(pipeline: &Config, dur: &Durability, plan_id: &str) -> Option<String> {
    let start = std::time::Instant::now();
    let (failing, attempted, skipped, recipe_label) = match &pipeline.recipe {
        Recipe::Shell(cmd) => {
            let res = run_shell(cmd, pipeline.timeout).await;
            (res, 1usize, 0usize, "shell")
        }
        Recipe::CargoAuto => {
            let mut failing: Option<String> = None;
            let mut attempted = 0usize;
            let mut skipped = 0usize;
            for step in CARGO_AUTO_STEPS {
                attempted += 1;
                match run_step(step, pipeline.timeout).await {
                    StepOutcome::Passed => {}
                    StepOutcome::Skipped => {
                        skipped += 1;
                    }
                    StepOutcome::Failed(reason) => {
                        failing = Some(reason);
                        break;
                    }
                }
            }
            (failing, attempted, skipped, "cargo:auto")
        }
    };

    let report = RunReport {
        recipe: recipe_label,
        ok: failing.is_none(),
        failing_step: failing
            .as_ref()
            .and_then(|r| r.split(':').next().map(|s| s.trim().to_string())),
        duration_ms: start.elapsed().as_millis(),
        steps_attempted: attempted,
        steps_skipped: skipped,
    };
    let _ = dur
        .audit()
        .append(EntityKind::Plan, plan_id, "pipeline.run", &report, None)
        .await;

    failing.map(|r| format!("pipeline_refused: {r}"))
}

/// True iff every task in `tasks` already carries a `pipeline_run`
/// evidence row whose payload `worktree_rev` matches the current
/// working-tree revision exported by the operator. Returning `true`
/// means Thor may skip re-running the configured pipeline.
pub(crate) async fn pre_validated(dur: &Durability, task_ids: &[String]) -> bool {
    let Some(expected_rev) = std::env::var(WORKTREE_REV_ENV).ok() else {
        return false;
    };
    let expected_rev = expected_rev.trim();
    if expected_rev.is_empty() || task_ids.is_empty() {
        return false;
    }
    for task_id in task_ids {
        let Ok(rows) = dur.evidence().list_by_task(task_id).await else {
            return false;
        };
        let mut matched = false;
        for ev in rows {
            if ev.kind != "pipeline_run" {
                continue;
            }
            if let Some(rev) = ev.payload.get("worktree_rev").and_then(|v| v.as_str()) {
                if rev == expected_rev {
                    matched = true;
                    break;
                }
            }
        }
        if !matched {
            return false;
        }
    }
    true
}

async fn run_shell(command: &str, dur_timeout: Duration) -> Option<String> {
    let child = match Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return Some(format!(
                "shell: pipeline `{command}` could not be invoked: {e}"
            ))
        }
    };
    finalize("shell", command, child, dur_timeout).await
}

async fn finalize(
    name: &str,
    label: &str,
    child: tokio::process::Child,
    dur_timeout: Duration,
) -> Option<String> {
    match timeout(dur_timeout, child.wait_with_output()).await {
        Ok(Ok(o)) if o.status.success() => None,
        Ok(Ok(o)) => Some(format!(
            "{name}: `{label}` failed (exit={}): {}",
            o.status
                .code()
                .map_or_else(|| "signal".to_string(), |code| code.to_string()),
            output_tail(&o.stdout, &o.stderr)
        )),
        Ok(Err(e)) => Some(format!("{name}: `{label}` could not be invoked: {e}")),
        Err(_) => Some(format!(
            "{name}: `{label}` timed out after {}",
            timeout_label(dur_timeout)
        )),
    }
}

pub(crate) fn timeout_label(value: Duration) -> String {
    if value.subsec_millis() == 0 {
        format!("{}s", value.as_secs())
    } else {
        format!("{}ms", value.as_millis())
    }
}

pub(crate) fn output_tail(stdout: &[u8], stderr: &[u8]) -> String {
    let mut output = Vec::with_capacity(stdout.len() + stderr.len());
    output.extend_from_slice(stdout);
    output.extend_from_slice(stderr);
    if output.len() <= PIPELINE_TAIL_BYTES {
        return String::from_utf8_lossy(&output).into_owned();
    }
    let start = output.len() - PIPELINE_TAIL_BYTES;
    format!(
        "[pipeline output truncated; showing last {PIPELINE_TAIL_BYTES} bytes]\n{}",
        String::from_utf8_lossy(&output[start..])
    )
}
