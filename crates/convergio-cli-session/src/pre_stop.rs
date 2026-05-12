//! `cvg session pre-stop` — end-of-session safety net.
//!
//! Hosts the dispatch surface for PRD-001 § Artefact 4: a [`Check`]
//! registry, a [`PreStopReport`] shape, and a runner that walks the
//! registry and prints a per-check verdict. The registry currently
//! mixes three implemented checks (`check.plan_pr_drift`,
//! `check.worktree.no_pr`, `check.friction.missing` — all
//! conservative, shell-first, sub-second) with three HTTP-shaped
//! [`CheckOutcome::NotImplemented`] stubs (`check.bus.inbound`,
//! `check.bus.outbound`, `check.handshake.uncommitted`). Each stub
//! is *advisory only*: [`CheckOutcome::blocks`] returns `false` for
//! it, so it cannot wedge a detach — it merely points operators at
//! the plan task that will promote it once the [`Check`] trait
//! widens to async.
//!
//! Keeping every check behind the same trait + registry means each
//! follow-up PR adds one file under `checks/` and registers
//! it here — no surgery on the dispatch loop, no risk of breaking the
//! command shape mid-rollout.
//!
//! ## Exit codes
//!
//! - `0` — every check passed (or every gap was acknowledged via
//!   `--force`). The agent may detach.
//! - `1` — at least one check produced findings AND `--force` was
//!   not set. The Stop hook should refuse.

use anyhow::Result;
use serde::Serialize;

/// Outcome of one safety check.
///
/// `Pass` and `Fail` are constructed by the dedicated check
/// implementations under `checks/` (W0b.2 follow-up tasks
/// `5298055b`, `564926dc`, `2c181be2`, `ab515d7e`, `95e6b262`,
/// `8dac18b9`). `#[allow(dead_code)]` is kept for now because not
/// every variant is constructed yet from real check impls — the
/// remaining three HTTP-shaped stubs still produce
/// `NotImplemented`.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum CheckOutcome {
    /// Check ran and found nothing to surface.
    Pass,
    /// Check ran and found one or more issues. `findings` is a
    /// human-readable list shown in the report.
    Fail {
        /// Short lines that explain what was found and where.
        findings: Vec<String>,
    },
    /// Check is scheduled but not yet implemented. Treated as a
    /// soft gap — does not block detach.
    NotImplemented {
        /// Plan task id of the dedicated implementation slot, so
        /// `cvg session pre-stop` output points the operator at
        /// the right next step.
        task_id: &'static str,
    },
}

impl CheckOutcome {
    /// Should this outcome block detach?
    pub fn blocks(&self) -> bool {
        matches!(self, CheckOutcome::Fail { .. })
    }
}

/// One safety check. Implementations live under
/// `checks/`; each one owns its own SQLite/git/gh queries.
pub trait Check: Send + Sync {
    /// Stable identifier shown in reports and logs (e.g. `"check.bus.inbound"`).
    fn id(&self) -> &'static str;
    /// Short human label for the report.
    fn label(&self) -> &'static str;
    /// Run the check. Implementations must be cheap (under a second
    /// in the common case) and must not write to the daemon.
    fn run(&self, ctx: &CheckContext) -> CheckOutcome;
}

/// Context passed to every check.
#[derive(Debug, Clone, Serialize)]
pub struct CheckContext {
    /// Stable agent identity supplied via `--agent-id`.
    pub agent_id: String,
    /// Daemon base URL, for checks that need to call HTTP routes.
    pub daemon_url: String,
}

/// Aggregate report.
#[derive(Debug, Serialize)]
pub struct PreStopReport {
    /// Effective agent identity.
    pub agent_id: String,
    /// Per-check outcomes, in registry order.
    pub results: Vec<CheckResult>,
    /// Whether `--force` was supplied.
    pub forced: bool,
}

/// One row in the report.
#[derive(Debug, Serialize)]
pub struct CheckResult {
    /// Mirrors [`Check::id`].
    pub id: &'static str,
    /// Mirrors [`Check::label`].
    pub label: &'static str,
    /// What the check produced.
    pub outcome: CheckOutcome,
}

/// Build the canonical registry.
///
/// `check_1_plan_pr_drift`, `worktree_no_pr` and `friction_missing`
/// are real (shell + curl, sub-second). The three remaining
/// HTTP-shaped checks (`bus.inbound`, `bus.outbound`,
/// `handshake.uncommitted`) ship as advisory `NotImplemented`
/// stubs — they never block detach (see [`CheckOutcome::blocks`])
/// and point the operator at the per-check task that will promote
/// each one once [`Check`] widens to async.
pub fn registry() -> Vec<Box<dyn Check>> {
    vec![
        Box::new(crate::checks::check_1_plan_pr_drift::PlanPrDriftCheck),
        Box::new(StubCheck {
            id: "check.bus.inbound",
            label: "inbound bus messages addressed to me, unconsumed",
            task_id: "564926dc",
        }),
        Box::new(StubCheck {
            id: "check.bus.outbound",
            label: "outbound stale bus messages sent by me, never consumed",
            task_id: "2c181be2",
        }),
        Box::new(crate::checks::worktree_no_pr::WorktreeNoPrCheck),
        Box::new(StubCheck {
            id: "check.handshake.uncommitted",
            label: "files declared in last bus handshake but never committed",
            task_id: "95e6b262",
        }),
        Box::new(crate::checks::friction_missing::FrictionMissingCheck),
    ]
}

/// Concrete `Check` returning [`CheckOutcome::NotImplemented`].
struct StubCheck {
    id: &'static str,
    label: &'static str,
    task_id: &'static str,
}

impl Check for StubCheck {
    fn id(&self) -> &'static str {
        self.id
    }
    fn label(&self) -> &'static str {
        self.label
    }
    fn run(&self, _ctx: &CheckContext) -> CheckOutcome {
        CheckOutcome::NotImplemented {
            task_id: self.task_id,
        }
    }
}

/// Run every check in the registry and produce a [`PreStopReport`].
pub fn run_pre_stop(ctx: &CheckContext, forced: bool) -> Result<PreStopReport> {
    let mut results = Vec::with_capacity(registry().len());
    for check in registry() {
        let outcome = check.run(ctx);
        results.push(CheckResult {
            id: check.id(),
            label: check.label(),
            outcome,
        });
    }
    Ok(PreStopReport {
        agent_id: ctx.agent_id.clone(),
        results,
        forced,
    })
}

/// True when the report has at least one outcome that should block
/// detach (i.e. a real `Fail` finding) AND `--force` was not used.
pub fn report_blocks_detach(report: &PreStopReport) -> bool {
    if report.forced {
        return false;
    }
    report.results.iter().any(|r| r.outcome.blocks())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> CheckContext {
        CheckContext {
            agent_id: "claude-code-roberdan".into(),
            daemon_url: "http://127.0.0.1:8420".into(),
        }
    }

    #[test]
    fn registry_lists_all_six_checks() {
        let reg = registry();
        assert_eq!(reg.len(), 6, "PRD-001 § Artefact 4 mandates six checks");
        let ids: Vec<&str> = reg.iter().map(|c| c.id()).collect();
        assert!(ids.contains(&"check.plan_pr_drift"));
        assert!(ids.contains(&"check.bus.inbound"));
        assert!(ids.contains(&"check.bus.outbound"));
        assert!(ids.contains(&"check.worktree.no_pr"));
        assert!(ids.contains(&"check.handshake.uncommitted"));
        assert!(ids.contains(&"check.friction.missing"));
    }

    #[test]
    fn stub_checks_still_point_at_their_task_ids() {
        // Three checks remain HTTP-shaped stubs after Check 1
        // (plan-vs-merged-PR drift) shipped — they must reference the
        // plan task that promotes them (operator clue).
        let report = run_pre_stop(&ctx(), false).expect("scaffold runs");
        let stub_ids: Vec<&'static str> = report
            .results
            .iter()
            .filter_map(|r| match &r.outcome {
                CheckOutcome::NotImplemented { task_id } => Some(*task_id),
                _ => None,
            })
            .collect();
        assert_eq!(stub_ids.len(), 3, "three checks remain HTTP-shaped stubs");
        for tid in stub_ids {
            assert!(!tid.is_empty(), "stub must point at a task");
        }
    }

    #[test]
    fn fail_outcome_blocks_unless_forced() {
        let mut report = run_pre_stop(&ctx(), false).expect("scaffold runs");
        report.results[0].outcome = CheckOutcome::Fail {
            findings: vec!["something to flag".into()],
        };
        assert!(report_blocks_detach(&report));
        report.forced = true;
        assert!(!report_blocks_detach(&report));
    }

    #[test]
    fn pass_outcome_does_not_block() {
        let mut report = run_pre_stop(&ctx(), false).expect("scaffold runs");
        for r in &mut report.results {
            r.outcome = CheckOutcome::Pass;
        }
        assert!(!report_blocks_detach(&report));
    }
}
