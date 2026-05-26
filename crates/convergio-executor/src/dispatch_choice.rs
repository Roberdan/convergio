//! Multi-vendor routing surface (W8 slice, ADR-0062).
//!
//! The full W8 workstream (5-7 days) adds a routing algorithm that
//! picks a runner per task by maximising `pass_rate / cost` under a
//! latency budget, using historical data from Smart Thor (W3) and
//! the model-evaluation framework (W10). Today the executor still
//! picks the runner kind the task carries (or the configured
//! default).
//!
//! This module ships the **audit surface** that every future routing
//! decision will need: one `dispatch.choice` row per spawn, capturing
//! the runner kind, profile, and a short rationale. With this in
//! place the routing algorithm in W8-full can replace the rationale
//! labels without changing the row shape, and operators can already
//! inspect "why was this runner chosen for that task?" today.

use convergio_durability::{audit::EntityKind, Durability, Task};
use convergio_runner::RunnerKind;
use serde::Serialize;
use tracing::warn;

/// Rationale labels for the routing decision. Forward-compatible —
/// W8-full will add variants like `pareto_winner`, `cost_floor`,
/// `latency_cap`, `cold_start`.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchRationale {
    /// Task carried a `runner_kind` override.
    TaskOverride,
    /// Task left runner unset; executor default was used.
    Default,
    /// Legacy `/bin/echo`-style spawn (no runner registry involvement).
    Legacy,
}

impl DispatchRationale {
    fn as_str(self) -> &'static str {
        match self {
            Self::TaskOverride => "task_override",
            Self::Default => "default",
            Self::Legacy => "legacy",
        }
    }
}

#[derive(Debug, Serialize)]
struct DispatchChoicePayload<'a> {
    runner_kind: String,
    profile: Option<&'a str>,
    rationale: &'a str,
    plan_id: &'a str,
}

/// Record a `dispatch.choice` audit row for a task. Logs and swallows
/// errors — the executor must not refuse to spawn just because the
/// audit emission failed.
pub(crate) async fn record_for_task(
    durability: &Durability,
    task: &Task,
    plan_id: &str,
    kind: &RunnerKind,
    legacy: bool,
) {
    let rationale = if legacy {
        DispatchRationale::Legacy
    } else if task.runner_kind.is_some() {
        DispatchRationale::TaskOverride
    } else {
        DispatchRationale::Default
    };
    let runner_kind = if legacy {
        "legacy-shell".to_string()
    } else {
        format!("{}:{}", kind.vendor, kind.model)
    };
    let payload = DispatchChoicePayload {
        runner_kind,
        profile: task.profile.as_deref(),
        rationale: rationale.as_str(),
        plan_id,
    };
    if let Err(e) = durability
        .audit()
        .append(
            EntityKind::Task,
            &task.id,
            "dispatch.choice",
            &payload,
            None,
        )
        .await
    {
        warn!(task_id = %task.id, plan_id, error = %e, "dispatch.choice audit append failed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rationale_strings_are_stable() {
        assert_eq!(DispatchRationale::TaskOverride.as_str(), "task_override");
        assert_eq!(DispatchRationale::Default.as_str(), "default");
        assert_eq!(DispatchRationale::Legacy.as_str(), "legacy");
    }
}
