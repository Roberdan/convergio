//! Planner errors.

use thiserror::Error;

/// Planner errors.
#[derive(Debug, Error)]
pub enum PlannerError {
    /// Mission text was empty after trimming.
    #[error("mission is empty")]
    EmptyMission,

    /// Underlying durability failure.
    #[error(transparent)]
    Durability(#[from] convergio_durability::DurabilityError),

    /// Spawning the `claude` CLI failed (binary missing, IO error).
    #[error("opus planner spawn failed: {0}")]
    OpusSpawn(String),

    /// `claude -p` exited with non-zero status.
    #[error("opus planner exited with status {status}: {stderr}")]
    OpusExited {
        /// Process exit code (-1 if killed by signal).
        status: i32,
        /// Captured stderr (truncated to 2 KB).
        stderr: String,
    },

    /// The opus output could not be decoded as the planner JSON
    /// schema. Either the model returned malformed JSON or it
    /// drifted from the schema.
    #[error("opus output is not valid plan JSON: {0}")]
    OpusOutputInvalid(String),
}

/// Convenience alias.
pub type Result<T, E = PlannerError> = std::result::Result<T, E>;
