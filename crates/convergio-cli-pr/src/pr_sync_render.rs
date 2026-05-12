//! Output renderers for `cvg pr sync`.
//!
//! Split out of `pr_sync.rs` so the orchestration loop in the
//! parent module stays under the 300-line per-crate cap
//! (CONSTITUTION § Agent context budget; audit finding LOW
//! pr_sync.rs:277).

use super::pr_sync::{SyncFailure, SyncOk, SyncReport, SyncSkip};
use super::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;
use serde_json::{json, Value};

/// Build the JSON body for a `SyncReport`. Kept separate from the
/// `render_report` printer so unit tests can assert on the
/// structure without going through stdout.
pub(super) fn report_json(report: &SyncReport) -> Value {
    json!({
        "scanned_prs": report.scanned_prs,
        "tracked_pairs": report.tracked_pairs,
        "transitioned": report.transitioned.iter().map(transitioned_json).collect::<Vec<_>>(),
        "skipped": report.skipped.iter().map(skipped_json).collect::<Vec<_>>(),
        "failed": report.failed.iter().map(failure_json).collect::<Vec<_>>(),
        "link_failures": report.link_failures.iter().map(failure_json).collect::<Vec<_>>(),
    })
}

fn transitioned_json(o: &SyncOk) -> Value {
    json!({
        "pr_number": o.pr_number,
        "task_id": o.task_id,
        "previous_status": o.previous_status,
    })
}

fn skipped_json(s: &SyncSkip) -> Value {
    json!({
        "pr_number": s.pr_number,
        "task_id": s.task_id,
        "current_status": s.current_status,
    })
}

fn failure_json(f: &SyncFailure) -> Value {
    json!({
        "pr_number": f.pr_number,
        "task_id": f.task_id,
        "reason": f.reason,
    })
}

pub(super) fn render_report(
    bundle: &Bundle,
    report: &SyncReport,
    output: OutputMode,
) -> Result<()> {
    match output {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&report_json(report))?);
        }
        OutputMode::Plain => render_plain(report),
        OutputMode::Human => render_human(bundle, report),
    }
    Ok(())
}

fn render_plain(r: &SyncReport) {
    println!(
        "scanned={} tracked={} transitioned={} skipped={} failed={} link_failures={}",
        r.scanned_prs,
        r.tracked_pairs,
        r.transitioned.len(),
        r.skipped.len(),
        r.failed.len(),
        r.link_failures.len()
    );
}

fn render_human(bundle: &Bundle, r: &SyncReport) {
    let scanned = r.scanned_prs.to_string();
    let tracked = r.tracked_pairs.to_string();
    println!(
        "{}",
        bundle.t(
            "pr-sync-header",
            &[("scanned", &scanned), ("tracked", &tracked)],
        )
    );
    println!();
    let tcount = r.transitioned.len().to_string();
    let header_key = if r.transitioned.is_empty() {
        "pr-sync-transitioned-empty"
    } else {
        "pr-sync-transitioned-header"
    };
    println!("  {}", bundle.t(header_key, &[("count", &tcount)]));
    for o in &r.transitioned {
        println!("    PR #{} -> task {}", o.pr_number, short_id(&o.task_id));
    }
    println!();
    let scount = r.skipped.len().to_string();
    println!(
        "  {}",
        bundle.t("pr-sync-skipped-header", &[("count", &scount)])
    );
    for s in &r.skipped {
        println!(
            "    PR #{} -> task {} ({})",
            s.pr_number,
            short_id(&s.task_id),
            s.current_status
        );
    }
    println!();
    let fcount = r.failed.len().to_string();
    println!(
        "  {}",
        bundle.t("pr-sync-failed-header", &[("count", &fcount)])
    );
    for f in &r.failed {
        println!(
            "    PR #{} -> task {}: {}",
            f.pr_number,
            short_id(&f.task_id),
            f.reason
        );
    }
    if !r.link_failures.is_empty() {
        println!();
        let lcount = r.link_failures.len().to_string();
        println!(
            "  {}",
            bundle.t("pr-sync-link-failures-header", &[("count", &lcount)])
        );
        for f in &r.link_failures {
            println!(
                "    PR #{} -> task {}: {}",
                f.pr_number,
                short_id(&f.task_id),
                f.reason
            );
        }
    }
}

fn short_id(id: &str) -> &str {
    if id.len() >= 8 {
        &id[..8]
    } else {
        id
    }
}
