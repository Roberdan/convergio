//! Step definitions and execution for the `cargo:auto` recipe (W3).
//!
//! Steps are either **mandatory** (fail-hard on any non-zero exit) or
//! **optional** (skip when the required binary is absent from PATH, fail-hard
//! otherwise). An optional step may also declare a `skip_if_no_match` flag:
//! when true, exit code 101 with stderr containing "no test target matches" is
//! treated as skipped rather than failed (Cargo's code for "no test matched
//! the filter").

use std::{process::Stdio, time::Duration};
use tokio::process::Command;

/// One step of a multi-step recipe.
pub(crate) struct Step {
    pub(crate) name: &'static str,
    pub(crate) program: &'static str,
    pub(crate) args: &'static [&'static str],
    /// If `true`, this step is allowed to be skipped when the binary is
    /// absent. Mandatory steps always fail-hard on spawn failure.
    pub(crate) optional: bool,
    /// Binary name to probe with `which` before running the step. When the
    /// probe binary is not found, the step is skipped. `None` for mandatory
    /// steps and for optional steps whose `program` is always on PATH (e.g.
    /// `cargo`) but whose subcommand plugin may be absent.
    pub(crate) probe: Option<&'static str>,
    /// When `true`, exit code 101 whose stderr contains "no test target
    /// matches" is treated as `Skipped` rather than `Failed`. Used for the
    /// i18n filter step where zero matching test files is not an error.
    pub(crate) skip_if_no_match: bool,
}

/// Outcome of executing one step.
pub(crate) enum StepOutcome {
    /// Step ran and succeeded.
    Passed,
    /// Step was skipped (optional + binary absent or 0 matching tests).
    Skipped,
    /// Step ran and failed; the string is the human-readable reason.
    Failed(String),
}

pub(crate) const CARGO_AUTO_STEPS: &[Step] = &[
    Step {
        name: "fmt",
        program: "cargo",
        args: &["fmt", "--all", "--check"],
        optional: false,
        probe: None,
        skip_if_no_match: false,
    },
    Step {
        name: "clippy",
        program: "cargo",
        args: &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
        optional: false,
        probe: None,
        skip_if_no_match: false,
    },
    Step {
        name: "test",
        program: "cargo",
        args: &["test", "--workspace"],
        optional: false,
        probe: None,
        skip_if_no_match: false,
    },
    // ── Optional steps (W3) ────────────────────────────────────────────────
    Step {
        name: "coverage",
        program: "cargo",
        args: &["llvm-cov", "--summary-only"],
        optional: true,
        probe: Some("cargo-llvm-cov"),
        skip_if_no_match: false,
    },
    Step {
        name: "a11y",
        program: "cvg",
        args: &["gates", "a11y", "--check"],
        optional: true,
        probe: Some("cvg"),
        skip_if_no_match: false,
    },
    Step {
        name: "i18n",
        program: "cargo",
        args: &["test", "--workspace", "--test", "*i18n*"],
        optional: true,
        probe: None,
        skip_if_no_match: true,
    },
];

/// Execute one step and return its outcome.
pub(crate) async fn run_step(step: &Step, dur_timeout: Duration) -> StepOutcome {
    // Probe: if a probe binary is specified and absent, skip.
    if step.optional {
        if let Some(probe) = step.probe {
            match Command::new("which")
                .arg(probe)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()
            {
                Ok(child) => {
                    if let Ok(status) = child.wait_with_output().await {
                        if !status.status.success() {
                            tracing::info!(step = step.name, probe, "skipped: cmd not found");
                            return StepOutcome::Skipped;
                        }
                    }
                }
                Err(_) => {
                    tracing::info!(step = step.name, probe, "skipped: cmd not found");
                    return StepOutcome::Skipped;
                }
            }
        }
    }

    // Run the step.
    let child = match Command::new(step.program)
        .args(step.args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(c) => c,
        Err(e) if step.optional => {
            tracing::info!(step = step.name, "skipped: cmd not found ({e})");
            return StepOutcome::Skipped;
        }
        Err(e) => {
            return StepOutcome::Failed(format!(
                "{}: `{}` could not be invoked: {e}",
                step.name, step.program
            ));
        }
    };

    let label = format!("{} {}", step.program, step.args.join(" "));
    match tokio::time::timeout(dur_timeout, child.wait_with_output()).await {
        Ok(Ok(o)) if o.status.success() => StepOutcome::Passed,
        Ok(Ok(o)) if step.skip_if_no_match && is_no_match_exit(&o) => {
            tracing::info!(step = step.name, "skipped: no test target matched filter");
            StepOutcome::Skipped
        }
        Ok(Ok(o)) => StepOutcome::Failed(format!(
            "{}: `{label}` failed (exit={}): {}",
            step.name,
            o.status
                .code()
                .map_or_else(|| "signal".to_string(), |c| c.to_string()),
            crate::pipeline::output_tail(&o.stdout, &o.stderr),
        )),
        Ok(Err(e)) => StepOutcome::Failed(format!(
            "{}: `{label}` could not be invoked: {e}",
            step.name
        )),
        Err(_) => StepOutcome::Failed(format!(
            "{}: `{label}` timed out after {}",
            step.name,
            crate::pipeline::timeout_label(dur_timeout),
        )),
    }
}

/// Returns `true` when the output matches Cargo's "no test target matches
/// pattern" exit (code 101 + stderr containing the sentinel text).
fn is_no_match_exit(o: &std::process::Output) -> bool {
    if o.status.code() != Some(101) {
        return false;
    }
    let stderr = std::str::from_utf8(&o.stderr).unwrap_or("");
    stderr.contains("no test target matches")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An optional step whose probe binary is guaranteed absent returns
    /// `Skipped` without failing.
    #[tokio::test]
    async fn optional_step_skipped_when_probe_absent() {
        let step = Step {
            name: "phantom",
            program: "this-binary-cannot-exist-convergio-w3",
            args: &[],
            optional: true,
            probe: Some("this-binary-cannot-exist-convergio-w3"),
            skip_if_no_match: false,
        };
        let outcome = run_step(&step, Duration::from_secs(5)).await;
        assert!(
            matches!(outcome, StepOutcome::Skipped),
            "optional step with absent probe must be Skipped"
        );
    }

    /// A mandatory step whose binary is absent returns `Failed`, not `Skipped`.
    #[tokio::test]
    async fn mandatory_step_fails_when_binary_absent() {
        let step = Step {
            name: "must-fail",
            program: "this-binary-cannot-exist-convergio-w3",
            args: &[],
            optional: false,
            probe: None,
            skip_if_no_match: false,
        };
        let outcome = run_step(&step, Duration::from_secs(5)).await;
        assert!(
            matches!(outcome, StepOutcome::Failed(_)),
            "mandatory step with absent binary must be Failed"
        );
    }
}
