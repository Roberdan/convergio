//! Routing decision entry point (W8, ADR-0062): map a task to a runner
//! kind + audit rationale, reading the `dispatch.choice` history via
//! [`super::load_stats`] and ranking with [`super::choose_runner`].

use super::{choose_runner, load_stats, Routed};
use crate::dispatch_choice::DispatchRationale;
use convergio_db::Pool;
use convergio_runner::RunnerKind;
use std::str::FromStr;

/// Decide the runner kind and audit rationale for a task. An explicit
/// `task.runner_kind` always wins (`TaskOverride`); otherwise the
/// pass_rate/cost algorithm picks from history (`ParetoWinner`),
/// falling back to `default_kind` on a cold start or read failure
/// (`Default`).
pub(crate) async fn decide(
    pool: &Pool,
    default_kind: &RunnerKind,
    task: &convergio_durability::Task,
) -> (RunnerKind, DispatchRationale) {
    if let Some(kind) = task
        .runner_kind
        .as_deref()
        .and_then(|s| RunnerKind::from_str(s).ok())
    {
        return (kind, DispatchRationale::TaskOverride);
    }
    let stats = match load_stats(pool).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "routing: failed to load dispatch history; using default");
            return (default_kind.clone(), DispatchRationale::Default);
        }
    };
    match choose_runner(&stats) {
        Routed::Pareto(kind) => (kind, DispatchRationale::ParetoWinner),
        Routed::ColdStart => (default_kind.clone(), DispatchRationale::Default),
    }
}
