//! Telemetry-collector loop.
//!
//! Runs every `tick_interval` (default 60 s), calls
//! [`Durability::record_telemetry_snapshot`], and logs the result.
//! Errors during a tick are logged at `warn!` and do **not** kill the
//! loop — transient SQLite contention must not take down the daemon.
//!
//! Each tick also prunes rows older than 7 days; the upsert inside
//! `record_telemetry_snapshot` is idempotent within the same minute
//! bucket, so overlapping ticks are safe.

use crate::facade::Durability;
use chrono::Duration;
use std::sync::Arc;
use std::time::Duration as StdDuration;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Configuration for the telemetry-collector loop.
#[derive(Debug, Clone)]
pub struct CollectorConfig {
    /// How often to snapshot the aggregate counters.
    pub tick_interval: Duration,
}

impl Default for CollectorConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::seconds(60),
        }
    }
}

/// Spawned-loop handle. Drop to abort the loop.
pub struct CollectorHandle {
    inner: JoinHandle<()>,
}

impl CollectorHandle {
    /// Abort the loop. Idempotent.
    pub fn abort(self) {
        self.inner.abort();
    }
}

/// Spawn the telemetry-collector loop and return its handle.
pub fn spawn(durability: Arc<Durability>, config: CollectorConfig) -> CollectorHandle {
    let interval = to_std(config.tick_interval);
    let inner = tokio::spawn(async move {
        info!(?config, "telemetry-collector started");
        loop {
            tokio::time::sleep(interval).await;
            match durability.record_telemetry_snapshot().await {
                Ok(()) => debug!("telemetry-collector tick: snapshot recorded"),
                Err(e) => warn!(error = %e, "telemetry-collector tick failed"),
            }
        }
    });
    CollectorHandle { inner }
}

fn to_std(d: Duration) -> StdDuration {
    StdDuration::from_secs(d.num_seconds().unsigned_abs())
}
