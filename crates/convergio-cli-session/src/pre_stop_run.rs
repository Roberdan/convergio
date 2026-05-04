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

/// Handle `cvg session pre-stop` from
/// [`crate::session::run`].
pub fn handle(client: &Client, output: OutputMode, agent_id: String, force: bool) -> Result<()> {
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
        OutputMode::Plain | OutputMode::Human => render_human(&agent_id, force, &report),
    }

    if report_blocks_detach(&report) {
        anyhow::bail!("session pre-stop reported findings; pass --force to detach anyway");
    }
    Ok(())
}

fn render_human(agent_id: &str, force: bool, report: &PreStopReport) {
    println!("session pre-stop report (agent_id={agent_id}, force={force})");
    for r in &report.results {
        let mark = match &r.outcome {
            CheckOutcome::Pass => "ok",
            CheckOutcome::Fail { .. } => "FAIL",
            CheckOutcome::NotImplemented { .. } => "todo",
        };
        println!("  [{mark}] {} — {}", r.id, r.label);
        if let CheckOutcome::Fail { findings } = &r.outcome {
            for f in findings {
                println!("        - {f}");
            }
        }
        if let CheckOutcome::NotImplemented { task_id } = &r.outcome {
            println!("        scheduled in plan task {task_id}");
        }
    }
}
