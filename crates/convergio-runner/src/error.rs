//! Errors surfaced by the runner.

use thiserror::Error;

/// Result alias.
pub type Result<T, E = RunnerError> = std::result::Result<T, E>;

/// Things that can go wrong while preparing or executing a runner.
#[derive(Debug, Error)]
pub enum RunnerError {
    /// `parse_kind` got a string it does not recognise.
    #[error("invalid runner kind: {0} (expected `<vendor>:<model>`)")]
    InvalidKind(String),

    /// The vendor CLI binary is missing from `PATH`.
    #[error("vendor CLI `{cli}` not on PATH; install it or add it to PATH")]
    CliMissing {
        /// Binary name (`claude` or `copilot`).
        cli: &'static str,
    },

    /// The vendor named in `RunnerKind` is not built-in and not
    /// declared in the runner registry (`~/.convergio/runners.toml`).
    /// ADR-0035 — add a `[vendors.<name>]` block or use a built-in.
    #[error("unknown vendor `{vendor}` — not a built-in (claude, copilot) and not declared in the runner registry")]
    UnknownVendor {
        /// Vendor tag from the wire-format `<vendor>:<model>`.
        vendor: String,
    },

    /// The model named in `RunnerKind` is not in the spec's
    /// `models` allowlist. The allowlist is opt-in — empty list
    /// means anything goes.
    #[error("model `{model}` is not in the allowlist for vendor `{vendor}` ({allowed:?})")]
    UnknownModel {
        /// Vendor tag.
        vendor: String,
        /// Rejected model name.
        model: String,
        /// Allowed models from the registry.
        allowed: Vec<String>,
    },

    /// `runners.toml` parse failure.
    #[error("invalid runners.toml: {0}")]
    RegistryInvalid(String),

    /// Subprocess execution failed before we could collect output.
    #[error(transparent)]
    Io(#[from] std::io::Error),

    /// Underlying durability error while loading task metadata.
    #[error(transparent)]
    Durability(#[from] convergio_durability::DurabilityError),

    /// Something else opaque.
    #[error("runner error: {0}")]
    Other(String),
}
