//! JSON schema the Opus planner emits and the parser consumes.
//!
//! Kept small + self-describing so the planner prompt can quote it
//! verbatim and so changes are visible in the diff.

use crate::error::{PlannerError, Result};
use serde::{Deserialize, Serialize};

/// Top-level plan shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanShape {
    /// Plan title — short, imperative.
    pub title: String,
    /// Optional plan description / motivation.
    #[serde(default)]
    pub description: Option<String>,
    /// Tasks — at least one, ordered by `(wave, sequence)`.
    pub tasks: Vec<TaskShape>,
}

/// One task in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskShape {
    /// Wave number — independent tasks share a wave; dependents go
    /// in subsequent waves. 1-indexed.
    pub wave: i64,
    /// Sequence within the wave. 1-indexed.
    pub sequence: i64,
    /// Imperative task title.
    pub title: String,
    /// Optional task body.
    #[serde(default)]
    pub description: Option<String>,
    /// Evidence the worker must attach (file paths, command names,
    /// test names). Empty list means "no specific artefact required".
    #[serde(default)]
    pub evidence_required: Vec<String>,
    /// Wire-format `<vendor>:<model>` (e.g. `claude:sonnet`).
    /// `None` falls back to the daemon-wide default.
    #[serde(default)]
    pub runner_kind: Option<String>,
    /// Permission profile (`standard` / `read_only` / `sandbox`).
    /// `None` falls back to the daemon-wide default.
    #[serde(default)]
    pub profile: Option<String>,
    /// Soft USD budget cap (forwarded to `claude --max-budget-usd`).
    #[serde(default)]
    pub max_budget_usd: Option<f32>,
}

impl PlanShape {
    /// Schema hint embedded in the planner prompt so the model
    /// emits the right shape. Kept short; full doc lives in the
    /// `///` comments above.
    pub const JSON_SCHEMA_HINT: &'static str = r#"{
  "title": "string",
  "description": "string | null",
  "tasks": [
    {
      "wave": 1,
      "sequence": 1,
      "title": "string",
      "description": "string | null",
      "evidence_required": ["string"],
      "runner_kind": "claude:sonnet | claude:opus | copilot:gpt-5.2 | <vendor>:<model>",
      "profile": "standard | read_only | sandbox",
      "max_budget_usd": 0.25
    }
  ]
}"#;

    /// Reject obvious schema drift. The planner trusts the model
    /// for the deeper semantic validation (e.g. wave dependencies).
    pub fn validate(&self) -> Result<()> {
        if self.title.trim().is_empty() {
            return Err(PlannerError::OpusOutputInvalid("empty title".into()));
        }
        if self.tasks.is_empty() {
            return Err(PlannerError::OpusOutputInvalid("no tasks emitted".into()));
        }
        for (i, t) in self.tasks.iter().enumerate() {
            if t.title.trim().is_empty() {
                return Err(PlannerError::OpusOutputInvalid(format!(
                    "task #{i}: empty title"
                )));
            }
            if t.wave < 1 {
                return Err(PlannerError::OpusOutputInvalid(format!(
                    "task #{i}: wave must be >= 1"
                )));
            }
            if t.sequence < 1 {
                return Err(PlannerError::OpusOutputInvalid(format!(
                    "task #{i}: sequence must be >= 1"
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shape_with(wave: i64, sequence: i64) -> PlanShape {
        PlanShape {
            title: "p".into(),
            description: None,
            tasks: vec![TaskShape {
                wave,
                sequence,
                title: "t".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            }],
        }
    }

    #[test]
    fn validate_accepts_one_indexed_wave_and_sequence() {
        shape_with(1, 1).validate().unwrap();
    }

    #[test]
    fn validate_rejects_wave_zero() {
        let err = shape_with(0, 1).validate().unwrap_err();
        assert!(matches!(err, PlannerError::OpusOutputInvalid(_)));
    }

    // Regression: schema documents sequence as 1-indexed but
    // validate() used to accept sequence == 0 (or negative) and
    // let it through to durability writes.
    #[test]
    fn validate_rejects_sequence_zero() {
        let err = shape_with(1, 0).validate().unwrap_err();
        let msg = format!("{err}");
        assert!(matches!(err, PlannerError::OpusOutputInvalid(_)));
        assert!(
            msg.contains("sequence"),
            "error must mention sequence: {msg}"
        );
    }

    #[test]
    fn validate_rejects_negative_sequence() {
        let err = shape_with(1, -1).validate().unwrap_err();
        assert!(matches!(err, PlannerError::OpusOutputInvalid(_)));
    }
}
