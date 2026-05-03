//! `Planner::solve` — turn a mission into a plan + tasks.
//!
//! ADR-0036: by default the planner is `claude:opus` running in
//! `--permission-mode plan` (read-only, vendor CLI only — ADR-0032).
//! When the `claude` binary is missing or the operator forces the
//! heuristic mode, the line-split fallback runs — it keeps unit
//! tests deterministic and CI green without a vendor login.

use crate::error::Result;
use crate::{heuristic, opus};
use convergio_durability::Durability;
use std::str::FromStr;

/// Which planner backend to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlannerMode {
    /// Use Opus when `claude` is on `PATH`; fall back to the
    /// line-split heuristic otherwise. The default.
    #[default]
    Auto,
    /// Force the Opus backend. Errors out when `claude` is missing.
    Opus,
    /// Always use the deterministic line-split heuristic.
    Heuristic,
}

impl PlannerMode {
    /// Resolve from `$CONVERGIO_PLANNER_MODE` (case-insensitive).
    /// Unknown / unset values map to `Auto`.
    pub fn from_env() -> Self {
        std::env::var("CONVERGIO_PLANNER_MODE")
            .ok()
            .as_deref()
            .and_then(|s| Self::from_str(s).ok())
            .unwrap_or_default()
    }
}

impl FromStr for PlannerMode {
    type Err = ();
    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        match s.to_ascii_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "opus" => Ok(Self::Opus),
            "heuristic" => Ok(Self::Heuristic),
            _ => Err(()),
        }
    }
}

/// Planner facade.
#[derive(Clone)]
pub struct Planner {
    durability: Durability,
    mode: PlannerMode,
}

impl Planner {
    /// Wrap a [`Durability`] facade with the env-derived mode.
    pub fn new(durability: Durability) -> Self {
        Self {
            durability,
            mode: PlannerMode::from_env(),
        }
    }

    /// Override the planner mode (used by tests + ops).
    pub fn with_mode(mut self, mode: PlannerMode) -> Self {
        self.mode = mode;
        self
    }

    /// Take a mission string, write a plan + tasks, return the
    /// plan id. The active backend depends on [`PlannerMode`].
    pub async fn solve(&self, mission: &str) -> Result<String> {
        match self.mode {
            PlannerMode::Heuristic => heuristic::solve(&self.durability, mission).await,
            PlannerMode::Opus => opus::solve(&self.durability, mission).await,
            PlannerMode::Auto => {
                if claude_on_path() {
                    opus::solve(&self.durability, mission).await
                } else {
                    heuristic::solve(&self.durability, mission).await
                }
            }
        }
    }
}

/// Cheap PATH probe — avoids the cost of actually spawning the
/// vendor CLI just to find out it is missing.
fn claude_on_path() -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|p| {
                let candidate = p.join("claude");
                candidate.is_file() || candidate.with_extension("exe").is_file()
            })
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_parses_known_values() {
        assert_eq!(PlannerMode::from_str("auto").unwrap(), PlannerMode::Auto);
        assert_eq!(PlannerMode::from_str("Opus").unwrap(), PlannerMode::Opus);
        assert_eq!(
            PlannerMode::from_str("HEURISTIC").unwrap(),
            PlannerMode::Heuristic
        );
        assert!(PlannerMode::from_str("nope").is_err());
    }
}
