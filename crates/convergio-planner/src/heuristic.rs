//! Line-split fallback planner.
//!
//! Splits the mission on newlines and creates one task per line, all
//! in wave 1 with no `runner_kind` / `profile` (those default to
//! daemon-wide settings at dispatch time). Used when the Opus
//! planner is unavailable (claude CLI missing) or when
//! `CONVERGIO_PLANNER_MODE=heuristic` forces the simple path —
//! e.g. in CI and unit tests.

use crate::error::{PlannerError, Result};
use convergio_durability::{Durability, NewPlan, NewTask};

/// Heuristic planner — no LLM, deterministic.
pub async fn solve(durability: &Durability, mission: &str) -> Result<String> {
    let lines: Vec<&str> = mission
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if lines.is_empty() {
        return Err(PlannerError::EmptyMission);
    }

    let title = lines[0].to_string();
    let description = if lines.len() > 1 {
        Some(lines[1..].join("\n"))
    } else {
        None
    };

    let plan = durability
        .create_plan(NewPlan {
            title,
            description,
            project: None,
        })
        .await?;

    for (i, line) in lines.iter().enumerate() {
        durability
            .create_task(
                &plan.id,
                NewTask {
                    wave: 1,
                    sequence: (i + 1) as i64,
                    title: (*line).to_string(),
                    description: None,
                    evidence_required: vec![],
                    runner_kind: None,
                    profile: None,
                    max_budget_usd: None,
                },
            )
            .await?;
    }

    Ok(plan.id)
}
