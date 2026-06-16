//! Multi-vendor routing algorithm (W8, ADR-0062).
//!
//! Picks the runner by maximising `score = pass_rate / cost` over the
//! runners seen in the `dispatch.choice` audit history:
//! - `pass_rate` = `done` / terminal-total of tasks dispatched to that
//!   runner kind (audit rows joined to the final `tasks.status`);
//! - `cost` is an operator weight from `CONVERGIO_RUNNER_COST_<KIND>`
//!   (default `1.0`).
//!
//! No history (cold start) ⇒ caller falls back to the daemon default.
//! The latency budget + Smart-Thor signals are W8-full; this is just
//! pass_rate/cost.

use convergio_db::Pool;
use convergio_durability::DurabilityError;
use convergio_runner::RunnerKind;
use std::collections::BTreeMap;
use std::str::FromStr;

/// Historical success stats for one runner kind, in `vendor:model`
/// wire form.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RunnerStats {
    /// Wire-format runner kind (`claude:sonnet`).
    pub kind: String,
    /// Tasks dispatched to this runner that reached `done`.
    pub done: u64,
    /// Total tasks dispatched to this runner that reached a terminal
    /// state (`done` or `failed`). Tasks still in flight are excluded
    /// so an unfinished run does not depress the rate.
    pub total: u64,
}

impl RunnerStats {
    /// Historical success fraction in `[0, 1]`; `0.0` when no terminal
    /// task exists yet (total-safe).
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            0.0
        } else {
            self.done as f64 / self.total as f64
        }
    }

    /// `pass_rate / cost`. `cost` is positive (see [`runner_cost`]).
    pub fn score(&self, cost: f64) -> f64 {
        self.pass_rate() / cost
    }
}

/// Read the per-runner cost weight from the environment.
///
/// The env name is the `vendor:model` wire string upper-cased with every
/// non-alphanumeric char → `_`, e.g. `claude:sonnet` →
/// `CONVERGIO_RUNNER_COST_CLAUDE_SONNET`. Unset, unparseable, or
/// non-positive values fall back to `1.0` (zero/negative would make the
/// score explode or flip sign).
pub(crate) fn runner_cost(kind: &str) -> f64 {
    let var = cost_env_var(kind);
    match std::env::var(&var) {
        Ok(raw) => match raw.trim().parse::<f64>() {
            Ok(c) if c.is_finite() && c > 0.0 => c,
            _ => {
                tracing::warn!(env = %var, raw = %raw, "runner cost not a positive number; using 1.0");
                1.0
            }
        },
        Err(_) => 1.0,
    }
}

/// Derive the `CONVERGIO_RUNNER_COST_<KIND>` env var name for a runner.
fn cost_env_var(kind: &str) -> String {
    let suffix: String = kind
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    format!("CONVERGIO_RUNNER_COST_{suffix}")
}

/// Outcome of a routing decision.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Routed {
    /// The algorithm picked a runner from history.
    Pareto(RunnerKind),
    /// No usable history — caller should use the daemon default.
    ColdStart,
}

/// Choose the runner with the highest `pass_rate / cost` over the
/// historical stats. Ties break on the lexicographically smallest
/// `vendor:model` so the choice is deterministic.
///
/// Returns [`Routed::ColdStart`] when `stats` is empty (no historical
/// dispatch) or when every candidate fails to parse — the caller then
/// keeps the existing default behaviour.
pub(crate) fn choose_runner(stats: &[RunnerStats]) -> Routed {
    // BTreeMap keeps deterministic iteration for stable tie-breaks.
    let mut best: Option<(f64, RunnerKind)> = None;
    let ranked: BTreeMap<&str, &RunnerStats> = stats.iter().map(|s| (s.kind.as_str(), s)).collect();
    for (kind_str, s) in ranked {
        let Ok(kind) = RunnerKind::from_str(kind_str) else {
            continue;
        };
        let score = s.score(runner_cost(kind_str));
        let better = match &best {
            None => true,
            Some((best_score, _)) => score > *best_score,
        };
        if better {
            best = Some((score, kind));
        }
    }
    match best {
        Some((_, kind)) => Routed::Pareto(kind),
        None => Routed::ColdStart,
    }
}

/// Load per-runner success stats from the `dispatch.choice` audit
/// history: each task's runner joined to its final `status`. Raw sqlx
/// on the executor pool — the same in-crate pattern as
/// `Executor::count_in_progress`, keeping the query in this crate (W8
/// scope). The `legacy-shell` pseudo-runner is excluded; it is not a
/// real routing target.
///
/// Only the **latest** `dispatch.choice` per task is counted (max
/// `seq`): a retried task is re-dispatched and may land on a different
/// runner, so attributing its single final outcome to every runner it
/// ever touched would credit the wrong runner with the result. The
/// denominator therefore counts terminal tasks (not raw dispatch rows),
/// which also keeps in-flight tasks from depressing the rate.
pub(crate) async fn load_stats(pool: &Pool) -> Result<Vec<RunnerStats>, DurabilityError> {
    // Pull (runner_kind, status) rows and aggregate in Rust: no
    // dependency on the SQLite JSON1 build, and trivially testable.
    let rows = sqlx::query_as::<_, (String, String)>(
        "SELECT a.payload, t.status \
         FROM audit_log a \
         JOIN tasks t ON t.id = a.entity_id \
         WHERE a.transition = 'dispatch.choice' \
           AND t.status IN ('done', 'failed') \
           AND a.seq = ( \
               SELECT MAX(a2.seq) FROM audit_log a2 \
               WHERE a2.entity_id = a.entity_id \
                 AND a2.transition = 'dispatch.choice' \
           )",
    )
    .fetch_all(pool.inner())
    .await
    .map_err(DurabilityError::from)?;

    let mut agg: BTreeMap<String, (u64, u64)> = BTreeMap::new();
    for (payload, status) in rows {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&payload) else {
            continue;
        };
        let Some(kind) = value.get("runner_kind").and_then(|v| v.as_str()) else {
            continue;
        };
        if kind == "legacy-shell" {
            continue;
        }
        let entry = agg.entry(kind.to_string()).or_insert((0, 0));
        entry.1 += 1; // total terminal
        if status == "done" {
            entry.0 += 1; // done
        }
    }

    Ok(agg
        .into_iter()
        .map(|(kind, (done, total))| RunnerStats { kind, done, total })
        .collect())
}

mod query;
pub(crate) use query::decide;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Env vars are process-global; serialise cost-touching cases.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn stat(kind: &str, done: u64, total: u64) -> RunnerStats {
        RunnerStats {
            kind: kind.into(),
            done,
            total,
        }
    }

    #[test]
    fn cost_env_var_normalizes_wire_string() {
        assert_eq!(
            cost_env_var("claude:sonnet"),
            "CONVERGIO_RUNNER_COST_CLAUDE_SONNET"
        );
        assert_eq!(
            cost_env_var("copilot:gpt-5.2"),
            "CONVERGIO_RUNNER_COST_COPILOT_GPT_5_2"
        );
    }

    #[test]
    fn pass_rate_and_score() {
        let s = stat("claude:sonnet", 3, 4);
        assert!((s.pass_rate() - 0.75).abs() < 1e-9);
        assert!((s.score(2.0) - 0.375).abs() < 1e-9);
    }

    #[test]
    fn cost_defaults_to_one_when_unset() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::remove_var("CONVERGIO_RUNNER_COST_QWEN_QWEN3_CODER");
        assert_eq!(runner_cost("qwen:qwen3-coder"), 1.0);
    }

    #[test]
    fn cost_rejects_non_positive() {
        let _g = ENV_LOCK.lock().unwrap();
        std::env::set_var("CONVERGIO_RUNNER_COST_CLAUDE_OPUS", "0");
        assert_eq!(runner_cost("claude:opus"), 1.0);
        std::env::set_var("CONVERGIO_RUNNER_COST_CLAUDE_OPUS", "-3");
        assert_eq!(runner_cost("claude:opus"), 1.0);
        std::env::set_var("CONVERGIO_RUNNER_COST_CLAUDE_OPUS", "2.5");
        assert_eq!(runner_cost("claude:opus"), 2.5);
        std::env::remove_var("CONVERGIO_RUNNER_COST_CLAUDE_OPUS");
    }

    #[test]
    fn empty_history_is_cold_start() {
        assert_eq!(choose_runner(&[]), Routed::ColdStart);
    }

    #[test]
    fn picks_highest_pass_rate_at_equal_cost() {
        let _g = ENV_LOCK.lock().unwrap();
        // No cost env set → all cost 1.0; highest pass_rate wins.
        let stats = vec![stat("claude:sonnet", 9, 10), stat("copilot:gpt-5.2", 5, 10)];
        assert_eq!(
            choose_runner(&stats),
            Routed::Pareto(RunnerKind::claude_sonnet())
        );
    }

    #[test]
    fn cost_can_flip_the_winner() {
        let _g = ENV_LOCK.lock().unwrap();
        // sonnet 0.9/1.0 = 0.9; opus 0.95 but cost 3 → 0.317.
        std::env::set_var("CONVERGIO_RUNNER_COST_CLAUDE_OPUS", "3");
        let stats = vec![stat("claude:sonnet", 9, 10), stat("claude:opus", 95, 100)];
        assert_eq!(
            choose_runner(&stats),
            Routed::Pareto(RunnerKind::claude_sonnet())
        );
        // Drop opus cost below the ratio and it wins.
        std::env::set_var("CONVERGIO_RUNNER_COST_CLAUDE_OPUS", "1.0");
        assert_eq!(
            choose_runner(&stats),
            Routed::Pareto(RunnerKind::claude_opus())
        );
        std::env::remove_var("CONVERGIO_RUNNER_COST_CLAUDE_OPUS");
    }

    #[test]
    fn ties_break_lexicographically() {
        let _g = ENV_LOCK.lock().unwrap();
        // Identical score → smallest vendor:model wins deterministically.
        let stats = vec![stat("copilot:gpt-5.2", 1, 1), stat("claude:sonnet", 1, 1)];
        assert_eq!(
            choose_runner(&stats),
            Routed::Pareto(RunnerKind::claude_sonnet())
        );
    }

    #[test]
    fn unparseable_kind_is_skipped() {
        let _g = ENV_LOCK.lock().unwrap();
        // "no-colon" cannot parse; the valid candidate still wins.
        let stats = vec![stat("no-colon", 1, 1), stat("claude:sonnet", 1, 2)];
        assert_eq!(
            choose_runner(&stats),
            Routed::Pareto(RunnerKind::claude_sonnet())
        );
    }
}
