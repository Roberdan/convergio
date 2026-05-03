//! `RunnerKind` and parsing.
//!
//! Wire format is `<vendor>:<model>` — e.g. `claude:sonnet`,
//! `claude:opus`, `copilot:gpt-5.2`, `qwen:qwen3-coder`. The string
//! round-trips through `Display` + `FromStr` so it is safe to store
//! in the `agents.kind` column without a lookup table.
//!
//! ADR-0035: the vendor is a free-form `String` so vendors declared
//! in the operator's `~/.convergio/runners.toml` resolve without a
//! Convergio recompile. Built-in vendors (`claude`, `copilot`) are
//! still recognised through the [`Family`] enum, used by the
//! hardcoded runners.

use crate::error::{Result, RunnerError};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Built-in vendor families with hardcoded runner implementations.
///
/// New vendors should be added through the runner registry (ADR-0035)
/// rather than this enum — the enum exists only so the two reference
/// implementations (`ClaudeRunner`, `CopilotRunner`) can be matched
/// without going through the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    /// Anthropic Claude Code CLI (`claude -p ...`).
    Claude,
    /// GitHub Copilot CLI (`copilot -p ...` aka `gh copilot`).
    Copilot,
}

impl Family {
    /// Binary name expected on `PATH`.
    pub fn cli(self) -> &'static str {
        match self {
            Family::Claude => "claude",
            Family::Copilot => "copilot",
        }
    }

    /// String tag used in the `<vendor>:<model>` wire format.
    pub fn tag(self) -> &'static str {
        match self {
            Family::Claude => "claude",
            Family::Copilot => "copilot",
        }
    }

    /// Reverse of [`Self::tag`]. `None` for vendors that need to
    /// resolve through the registry.
    pub fn from_tag(tag: &str) -> Option<Self> {
        match tag {
            "claude" => Some(Family::Claude),
            "copilot" => Some(Family::Copilot),
            _ => None,
        }
    }
}

/// Concrete runner = vendor + model.
///
/// `vendor` is a free-form string. When it matches a [`Family`] tag
/// the executor uses the hardcoded runner; otherwise it looks up
/// the vendor in the registry (`~/.convergio/runners.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunnerKind {
    /// Vendor identifier (e.g. `claude`, `copilot`, `qwen`,
    /// `codex`). Free-form so the registry can supply unknowns.
    pub vendor: String,
    /// Model passed to the vendor CLI. Vendor-specific naming
    /// (`sonnet` / `opus` for Claude, `gpt-5.2` for Copilot,
    /// `qwen3-coder` for Qwen). Unknown strings are forwarded as-is
    /// so new models surface without a Convergio release.
    pub model: String,
}

impl RunnerKind {
    /// Build an explicit kind without going through the wire string.
    pub fn new(vendor: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            vendor: vendor.into(),
            model: model.into(),
        }
    }

    /// Default Claude (sonnet) — the cheap, fast option.
    pub fn claude_sonnet() -> Self {
        Self::new("claude", "sonnet")
    }

    /// Claude Opus — for tasks the planner flags as harder.
    pub fn claude_opus() -> Self {
        Self::new("claude", "opus")
    }

    /// Copilot with GitHub's default GPT model.
    pub fn copilot_gpt() -> Self {
        Self::new("copilot", "gpt-5.2")
    }

    /// Resolve to a built-in [`Family`], if the vendor name matches
    /// one of the two reference implementations. Custom vendors
    /// return `None` and resolve through the registry instead.
    pub fn family(&self) -> Option<Family> {
        Family::from_tag(&self.vendor)
    }
}

impl fmt::Display for RunnerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.vendor, self.model)
    }
}

impl FromStr for RunnerKind {
    type Err = RunnerError;
    fn from_str(s: &str) -> Result<Self> {
        let (vendor, model) = s
            .split_once(':')
            .ok_or_else(|| RunnerError::InvalidKind(s.to_string()))?;
        if vendor.is_empty() || model.is_empty() {
            return Err(RunnerError::InvalidKind(s.to_string()));
        }
        Ok(Self::new(vendor, model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_claude_sonnet() {
        let k: RunnerKind = "claude:sonnet".parse().unwrap();
        assert_eq!(k.vendor, "claude");
        assert_eq!(k.model, "sonnet");
        assert_eq!(k.family(), Some(Family::Claude));
        assert_eq!(k.to_string(), "claude:sonnet");
    }

    #[test]
    fn round_trip_copilot_gpt() {
        let k: RunnerKind = "copilot:gpt-5.2".parse().unwrap();
        assert_eq!(k.family(), Some(Family::Copilot));
        assert_eq!(k.model, "gpt-5.2");
    }

    #[test]
    fn unknown_vendor_parses_with_no_family() {
        let k: RunnerKind = "qwen:qwen3-coder".parse().unwrap();
        assert_eq!(k.vendor, "qwen");
        assert_eq!(k.model, "qwen3-coder");
        assert_eq!(k.family(), None);
    }

    #[test]
    fn empty_model_is_rejected() {
        let err = "claude:".parse::<RunnerKind>().unwrap_err();
        assert!(matches!(err, RunnerError::InvalidKind(_)));
    }

    #[test]
    fn empty_vendor_is_rejected() {
        let err = ":sonnet".parse::<RunnerKind>().unwrap_err();
        assert!(matches!(err, RunnerError::InvalidKind(_)));
    }

    #[test]
    fn missing_separator_is_rejected() {
        let err = "claude".parse::<RunnerKind>().unwrap_err();
        assert!(matches!(err, RunnerError::InvalidKind(_)));
    }

    #[test]
    fn family_cli_is_stable() {
        assert_eq!(Family::Claude.cli(), "claude");
        assert_eq!(Family::Copilot.cli(), "copilot");
    }
}
