//! Private helpers for the supervisor (timeout wrappers).

use crate::error::{LifecycleError, Result};
use std::time::{Duration as StdDuration, Instant};

pub(super) async fn timeout_query<E>(
    command: &str,
    timeout: StdDuration,
    started: Instant,
    query: E,
) -> Result<sqlx::sqlite::SqliteQueryResult>
where
    E: std::future::Future<
        Output = std::result::Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error>,
    >,
{
    let remaining = timeout
        .checked_sub(started.elapsed())
        .filter(|d| !d.is_zero())
        .ok_or_else(|| spawn_timeout(command, timeout))?;
    tokio::time::timeout(remaining, query)
        .await
        .map_err(|_| spawn_timeout(command, timeout))?
        .map_err(LifecycleError::from)
}

pub(super) fn spawn_timeout(command: &str, timeout: StdDuration) -> LifecycleError {
    LifecycleError::SpawnTimedOut {
        command: command.to_string(),
        timeout_ms: timeout.as_millis(),
    }
}
