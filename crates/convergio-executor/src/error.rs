//! Executor errors.

use thiserror::Error;

/// Errors the executor can produce.
#[derive(Debug, Error)]
pub enum ExecutorError {
    /// Layer 1 failure.
    #[error(transparent)]
    Durability(#[from] convergio_durability::DurabilityError),

    /// Layer 3 failure.
    #[error(transparent)]
    Lifecycle(#[from] convergio_lifecycle::LifecycleError),

    /// Runner-crate failure during argv assembly (e.g. invalid
    /// `runner_kind` or missing vendor CLI).
    #[error(transparent)]
    Runner(#[from] convergio_runner::RunnerError),
}

/// Convenience alias.
pub type Result<T, E = ExecutorError> = std::result::Result<T, E>;
