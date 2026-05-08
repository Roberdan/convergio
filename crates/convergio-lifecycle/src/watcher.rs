//! OS-watcher loop.
//!
//! Polls every `running` row in `agent_processes` and asks the OS
//! whether the PID is still alive. When it isn't, the row flips to
//! `exited` (no exit code recorded — POSIX `kill -0` doesn't tell us
//! that). Future work: keep a tokio `Child` handle keyed by row id and
//! await its real exit status.
//!
//! Today this loop closes the obvious gap that nothing flips a row
//! out of `running` without a manual `mark_exited` call. The Layer 1
//! reaper still owns the *task-level* recovery (releases the task back
//! to `pending`); the OS-watcher owns the *process-level* status.

use crate::error::Result;
use crate::supervisor::Supervisor;
use chrono::Duration;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

/// Watcher configuration.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// How often the loop ticks.
    pub tick_interval: Duration,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            tick_interval: Duration::seconds(30),
        }
    }
}

/// Spawned-loop handle. Drop the handle to abort.
pub struct WatcherHandle {
    inner: JoinHandle<()>,
}

impl WatcherHandle {
    /// Abort the loop. Idempotent.
    pub fn abort(self) {
        self.inner.abort();
    }
}

/// Spawn the watcher loop.
///
/// Errors during a tick are logged at `warn!` and do not kill the
/// loop — same discipline as the Layer 1 reaper.
pub fn spawn(supervisor: Supervisor, config: WatcherConfig) -> WatcherHandle {
    let inner = tokio::spawn(async move {
        info!(?config, "lifecycle watcher started");
        let interval = tokio_duration(config.tick_interval);
        loop {
            tokio::time::sleep(interval).await;
            match tick(&supervisor).await {
                Ok(n) if n > 0 => info!(reaped = n, "watcher tick"),
                Ok(_) => debug!("watcher tick: nothing dead"),
                Err(e) => warn!(error = %e, "watcher tick failed"),
            }
        }
    });
    WatcherHandle { inner }
}

/// Run one tick. Returns the number of processes flipped to `exited`.
///
/// Exposed for tests and for callers who want to drive the loop on
/// their own schedule.
pub async fn tick(supervisor: &Supervisor) -> Result<usize> {
    let candidates = sqlx::query_as::<_, (String, Option<i64>)>(
        "SELECT id, pid FROM agent_processes WHERE status = 'running'",
    )
    .fetch_all(supervisor.pool().inner())
    .await?;

    let mut flipped = 0usize;
    for (id, pid) in candidates {
        let alive = match pid {
            Some(p) => is_alive(p),
            None => false,
        };
        if !alive {
            // No exit code — the OS doesn't tell us via kill(0).
            supervisor.mark_exited(&id, None, true).await?;
            flipped += 1;
        }
    }
    Ok(flipped)
}

/// `kill -0 <pid>`. Returns true if the process exists (or we lack
/// permission to signal it, which still implies it exists).
///
/// Public so the executor's heartbeat sidecar can poll the same way
/// the watcher does without re-implementing the POSIX dance.
#[cfg(unix)]
pub fn is_alive(pid: i64) -> bool {
    use nix::errno::Errno;
    use nix::sys::signal::kill;
    use nix::unistd::Pid;

    if pid <= 0 {
        return false;
    }
    let p = Pid::from_raw(pid as i32);
    match kill(p, None) {
        Ok(()) => true,
        Err(Errno::ESRCH) => false,
        // EPERM and friends mean the process exists but we can't
        // signal it — still alive.
        Err(_) => true,
    }
}

#[cfg(not(unix))]
pub fn is_alive(_pid: i64) -> bool {
    // Windows path: not in MVP scope. Fall back to "still running" so
    // we don't accidentally flip everything to exited on Windows.
    true
}

fn tokio_duration(d: Duration) -> std::time::Duration {
    std::time::Duration::from_millis(d.num_milliseconds().max(1) as u64)
}
