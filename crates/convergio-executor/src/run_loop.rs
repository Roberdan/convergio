//! Background loop driver for the executor.
//!
//! Split out from `executor.rs` so the dispatcher itself stays
//! under the 300-line cap. Owns nothing except the tokio task that
//! ticks the executor on the configured interval and the abort
//! handle the daemon hangs onto.

use crate::executor::Executor;
use chrono::Duration;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Spawned-loop handle. Drop the handle to abort.
pub struct ExecutorHandle {
    inner: JoinHandle<()>,
}

impl ExecutorHandle {
    /// Abort the loop. Idempotent.
    pub fn abort(&self) {
        self.inner.abort();
    }
}

/// Spawn the executor loop. Errors during a tick are logged at
/// `warn!` and do not kill the loop — the next tick retries.
pub fn spawn_loop(executor: Arc<Executor>, tick_interval: Duration) -> ExecutorHandle {
    let inner = tokio::spawn(async move {
        info!(
            tick_secs = tick_interval.num_seconds(),
            "executor loop started"
        );
        let interval = tokio_duration(tick_interval);
        loop {
            tokio::time::sleep(interval).await;
            match executor.tick().await {
                Ok(n) if n > 0 => info!(dispatched = n, "executor tick"),
                Ok(_) => debug!("executor tick: nothing pending"),
                Err(e) => warn!(error = %e, "executor tick failed"),
            }
        }
    });
    ExecutorHandle { inner }
}

fn tokio_duration(d: Duration) -> std::time::Duration {
    std::time::Duration::from_millis(d.num_milliseconds().max(1) as u64)
}
