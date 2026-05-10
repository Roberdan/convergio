//! Evidence kind `usage` schema.
//!
//! The payload is attached by runners/hosts to report LLM usage:
//! tokens in/out, model label, and incremental USD cost.

use crate::error::{DurabilityError, Result};
use serde::Deserialize;

/// Payload schema for `evidence.kind == "usage"`.
#[derive(Debug, Clone, Deserialize)]
pub struct UsageEvidence {
    /// Input (prompt) tokens for this operation.
    pub input_tokens: i64,
    /// Output (completion) tokens for this operation.
    pub output_tokens: i64,
    /// Model label in `<vendor>:<model>` wire format.
    pub model: String,
    /// Incremental USD cost for this operation.
    pub cost_usd: f64,
}

impl UsageEvidence {
    /// Total tokens attributed by this payload.
    pub fn total_tokens(&self) -> i64 {
        self.input_tokens.saturating_add(self.output_tokens)
    }

    /// Validate basic shape and ranges.
    pub fn validate(&self) -> Result<()> {
        if self.input_tokens < 0 {
            return Err(DurabilityError::InvalidEvidence {
                reason: "usage.input_tokens must be >= 0".into(),
            });
        }
        if self.output_tokens < 0 {
            return Err(DurabilityError::InvalidEvidence {
                reason: "usage.output_tokens must be >= 0".into(),
            });
        }
        if self.model.trim().is_empty() {
            return Err(DurabilityError::InvalidEvidence {
                reason: "usage.model must be non-empty".into(),
            });
        }
        if !self.cost_usd.is_finite() || self.cost_usd < 0.0 {
            return Err(DurabilityError::InvalidEvidence {
                reason: "usage.cost_usd must be a finite number >= 0".into(),
            });
        }
        Ok(())
    }
}

/// Decode and validate a `usage` payload.
pub fn parse_usage(payload: serde_json::Value) -> Result<UsageEvidence> {
    let usage: UsageEvidence = serde_json::from_value(payload)?;
    usage.validate()?;
    Ok(usage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_usage_accepts_valid_payload() {
        let usage = parse_usage(json!({
            "input_tokens": 10,
            "output_tokens": 20,
            "model": "copilot:gpt-5.2",
            "cost_usd": 0.001
        }))
        .expect("valid usage");
        assert_eq!(usage.total_tokens(), 30);
    }

    #[test]
    fn parse_usage_rejects_negative_tokens() {
        let err = parse_usage(json!({
            "input_tokens": -1,
            "output_tokens": 0,
            "model": "x",
            "cost_usd": 0.0
        }))
        .unwrap_err();
        assert!(matches!(err, DurabilityError::InvalidEvidence { .. }));
    }
}
