//! Output renderers for [`crate::handshake::Report`].
//!
//! Two modes: localized human (`Bundle.t`) and plain key=value pairs
//! for shell pipelines. JSON rendering stays inline in `handshake.rs`
//! (it is a one-liner via `serde_json::to_string_pretty`).

use crate::handshake::{PhaseOutcome, Report};
use convergio_i18n::Bundle;

/// Render the localized human view to stdout.
pub(crate) fn render_human(report: &Report, bundle: &Bundle) {
    println!(
        "{}",
        bundle.t(
            "coherence-handshake-summary",
            &[
                ("daemon", &report.daemon),
                ("timeout", &report.timeout_ms.to_string()),
            ]
        )
    );
    for p in &report.phases {
        // Phase 0 is the bootstrap (plan creation) failure path —
        // p.label already carries the literal "bootstrap"; only
        // localized 1..6 phase names go through Fluent. Without this
        // branch a bootstrap fail mislabels as "register A+B".
        let label = if p.n == 0 {
            p.label.clone()
        } else {
            let n = p.n.clamp(1, 6);
            let key = format!("coherence-handshake-phase-{n}");
            bundle.t(&key, &[])
        };
        let icon = phase_icon(p.outcome);
        println!(
            "  {icon} phase {} ({label}): {} [{}ms]",
            p.n, p.detail, p.elapsed_ms
        );
    }
    let key = if report.success {
        "coherence-handshake-success"
    } else if report
        .phases
        .iter()
        .any(|p| p.outcome == PhaseOutcome::Timeout)
    {
        "coherence-handshake-timeout"
    } else {
        "coherence-handshake-fail"
    };
    println!(
        "{}",
        bundle.t(
            key,
            &[
                ("elapsed", &report.total_elapsed_ms.to_string()),
                ("timeout", &report.timeout_ms.to_string()),
            ]
        )
    );
}

/// Render the plain key=value view to stdout.
pub(crate) fn render_plain(report: &Report) {
    println!(
        "daemon={} success={} elapsed_ms={} timeout_ms={}",
        report.daemon, report.success, report.total_elapsed_ms, report.timeout_ms
    );
    for p in &report.phases {
        println!(
            "phase={} label={} outcome={} elapsed_ms={} detail={}",
            p.n,
            p.label,
            outcome_key(p.outcome),
            p.elapsed_ms,
            p.detail
        );
    }
}

fn phase_icon(o: PhaseOutcome) -> &'static str {
    match o {
        PhaseOutcome::Ok => "ok",
        PhaseOutcome::Timeout => "timeout",
        PhaseOutcome::Failed => "fail",
        PhaseOutcome::Skipped => "skip",
    }
}

fn outcome_key(o: PhaseOutcome) -> &'static str {
    match o {
        PhaseOutcome::Ok => "ok",
        PhaseOutcome::Timeout => "timeout",
        PhaseOutcome::Failed => "failed",
        PhaseOutcome::Skipped => "skipped",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_key_covers_all_variants() {
        assert_eq!(outcome_key(PhaseOutcome::Ok), "ok");
        assert_eq!(outcome_key(PhaseOutcome::Timeout), "timeout");
        assert_eq!(outcome_key(PhaseOutcome::Failed), "failed");
        assert_eq!(outcome_key(PhaseOutcome::Skipped), "skipped");
    }

    #[test]
    fn phase_icon_distinguishes_outcomes() {
        assert_ne!(
            phase_icon(PhaseOutcome::Ok),
            phase_icon(PhaseOutcome::Failed)
        );
        assert_ne!(
            phase_icon(PhaseOutcome::Timeout),
            phase_icon(PhaseOutcome::Skipped)
        );
    }
}
