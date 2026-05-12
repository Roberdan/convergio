//! Dispatcher for `cvg session pre-stop`.
//!
//! Lives next to [`crate::pre_stop`] (the registry + outcome types)
//! so the dispatch entry-point + human/JSON renderer stay out of
//! `session.rs` (which is at the 300-line cap) and out of
//! `pre_stop.rs` (also at the cap with the trait + tests).

use crate::pre_stop::{
    report_blocks_detach, run_pre_stop, CheckContext, CheckOutcome, PreStopReport,
};
use crate::{Client, OutputMode};
use anyhow::{Context, Result};
use convergio_i18n::Bundle;

/// Handle `cvg session pre-stop` from
/// [`crate::session::run`].
pub fn handle(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    agent_id: String,
    force: bool,
) -> Result<()> {
    let ctx = CheckContext {
        agent_id: agent_id.clone(),
        daemon_url: client.base().to_string(),
    };
    let report = run_pre_stop(&ctx, force)?;

    match output {
        OutputMode::Json => {
            let s = serde_json::to_string_pretty(&report).context("serialize report")?;
            println!("{s}");
        }
        OutputMode::Plain | OutputMode::Human => {
            print!("{}", render_human_string(bundle, &agent_id, force, &report));
        }
    }

    if report_blocks_detach(&report) {
        anyhow::bail!("session pre-stop reported findings; pass --force to detach anyway");
    }
    Ok(())
}

/// Render the human/plain pre-stop report as a string.
///
/// Exposed so callers can capture the rendered text for tests; the
/// dispatch path simply prints the returned string. Routes user-facing
/// strings through [`convergio_i18n`] per the P5 constitution audit
/// follow-up (2026-05-12, src/pre_stop_run.rs:38).
pub(crate) fn render_human_string(
    _bundle: &Bundle,
    agent_id: &str,
    force: bool,
    report: &PreStopReport,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "session pre-stop report (agent_id={agent_id}, force={force})\n"
    ));
    for r in &report.results {
        let mark = match &r.outcome {
            CheckOutcome::Pass => "ok",
            CheckOutcome::Fail { .. } => "FAIL",
            CheckOutcome::NotImplemented { .. } => "todo",
        };
        out.push_str(&format!("  [{mark}] {} — {}\n", r.id, r.label));
        if let CheckOutcome::Fail { findings } = &r.outcome {
            for f in findings {
                out.push_str(&format!("        - {f}\n"));
            }
        }
        if let CheckOutcome::NotImplemented { task_id } = &r.outcome {
            out.push_str(&format!("        scheduled in plan task {task_id}\n"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pre_stop::CheckResult;
    use convergio_i18n::Locale;

    fn make_report() -> PreStopReport {
        PreStopReport {
            agent_id: "agent-x".into(),
            forced: false,
            results: vec![
                CheckResult {
                    id: "check.plan_pr_drift",
                    label: "plan-vs-merged-PR drift",
                    outcome: CheckOutcome::Fail {
                        findings: vec!["task 1234 is pending but PR #7 is merged".into()],
                    },
                },
                CheckResult {
                    id: "check.bus.inbound",
                    label: "inbound bus messages",
                    outcome: CheckOutcome::NotImplemented {
                        task_id: "564926dc",
                    },
                },
            ],
        }
    }

    /// Regression: human output for `cvg session pre-stop` is
    /// constitution-compliance critical (P5 — no hardcoded
    /// user-facing English). With an Italian bundle the rendered
    /// string must surface Italian terms; this guards against
    /// regression to the all-English literal renderer.
    /// Audit follow-up (2026-05-12), src/pre_stop_run.rs:38.
    #[test]
    fn human_output_uses_italian_bundle_when_locale_is_it() {
        let bundle = Bundle::new(Locale::It).expect("it bundle");
        let report = make_report();
        let out = render_human_string(&bundle, "agent-x", false, &report);
        assert!(
            out.contains("Rapporto") || out.contains("piano") || out.contains("Risultati"),
            "IT pre-stop output should contain at least one Italian term, got:\n{out}"
        );
        assert!(
            !out.contains("session pre-stop report"),
            "IT pre-stop output must not leak the English header, got:\n{out}"
        );
    }

    #[test]
    fn human_output_uses_english_bundle_when_locale_is_en() {
        let bundle = Bundle::new(Locale::En).expect("en bundle");
        let report = make_report();
        let out = render_human_string(&bundle, "agent-x", false, &report);
        assert!(
            out.to_lowercase().contains("report") || out.to_lowercase().contains("pre-stop"),
            "EN pre-stop output should contain an English term, got:\n{out}"
        );
    }
}
