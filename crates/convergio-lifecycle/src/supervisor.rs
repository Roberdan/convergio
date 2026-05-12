//! Supervisor — spawns processes and tracks them in the DB.

use crate::error::{LifecycleError, Result};
use crate::model::{parse_ts, parse_ts_opt, AgentProcess, ProcessStatus, SpawnSpec};
use crate::supervisor_helpers::{spawn_timeout, timeout_query};
use chrono::Utc;
use convergio_db::Pool;
use std::process::Stdio;
use std::time::{Duration as StdDuration, Instant};
use tokio::process::Command;
use tracing::warn;
use uuid::Uuid;

const DEFAULT_SPAWN_TIMEOUT: StdDuration = StdDuration::from_secs(10);

/// Read/write access to `agent_processes` + spawn capability.
#[derive(Clone)]
pub struct Supervisor {
    pool: Pool,
    bus: Option<convergio_bus::Bus>,
}

impl Supervisor {
    /// Wrap a pool. The caller is responsible for having run
    /// [`crate::init`] at least once.
    pub fn new(pool: Pool) -> Self {
        Self { pool, bus: None }
    }

    /// Return a supervisor that relays process stdout to the plan bus.
    pub fn new_with_bus(pool: Pool, bus: convergio_bus::Bus) -> Self {
        Self {
            pool,
            bus: Some(bus),
        }
    }

    /// Borrow the underlying pool — needed by the watcher loop.
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Spawn a process and persist its row using the default timeout.
    ///
    /// The function returns as soon as the OS PID is captured. The
    /// process keeps running detached; the watcher updates `status` on
    /// exit. The timeout bounds async DB bookkeeping around spawn. OS
    /// process creation itself is synchronous and cannot be interrupted
    /// safely on every platform, so a blocked `spawn` may only be
    /// observed after control returns.
    pub async fn spawn(&self, spec: SpawnSpec) -> Result<AgentProcess> {
        self.spawn_with_timeout(spec, DEFAULT_SPAWN_TIMEOUT).await
    }

    /// Spawn a process with an explicit bookkeeping timeout.
    ///
    /// If a timeout happens after the child exists but before the PID is
    /// durably recorded, the supervisor kills the child rather than
    /// leaving an untracked local process behind.
    pub async fn spawn_with_timeout(
        &self,
        spec: SpawnSpec,
        timeout: StdDuration,
    ) -> Result<AgentProcess> {
        if timeout.is_zero() {
            return Err(spawn_timeout(&spec.command, timeout));
        }
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let now_str = now.to_rfc3339();
        let started = Instant::now();

        let insert = sqlx::query(
            "INSERT INTO agent_processes (id, kind, command, plan_id, task_id, pid, \
             status, exit_code, last_heartbeat_at, started_at, ended_at) \
             VALUES (?, ?, ?, ?, ?, NULL, 'starting', NULL, NULL, ?, NULL)",
        )
        .bind(&id)
        .bind(&spec.kind)
        .bind(&spec.command)
        .bind(&spec.plan_id)
        .bind(&spec.task_id)
        .bind(&now_str)
        .execute(self.pool.inner());
        timeout_query(&spec.command, timeout, started, insert).await?;

        let mut cmd = Command::new(&spec.command);
        cmd.args(&spec.args);
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        if let Some(cwd) = &spec.cwd {
            cmd.current_dir(cwd);
        }
        // Pipe stdin only when the caller actually has a payload to
        // write — vendor-CLI runners (claude, copilot, qwen) read the
        // prompt from stdin. Otherwise keep the legacy null stdin.
        if spec.stdin_payload.is_some() {
            cmd.stdin(Stdio::piped());
        } else {
            cmd.stdin(Stdio::null());
        }
        let pipe_stdout = self.bus.is_some() && spec.plan_id.is_some();
        if pipe_stdout {
            cmd.stdout(Stdio::piped());
        } else {
            cmd.stdout(Stdio::null());
        }
        cmd.stderr(Stdio::null());

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                self.record_spawn_failed(&id).await;
                return Err(LifecycleError::SpawnFailed(format!(
                    "{}: {e}",
                    spec.command
                )));
            }
        };

        if started.elapsed() >= timeout {
            self.kill_unrecorded_child(&mut child).await;
            self.record_spawn_failed(&id).await;
            return Err(spawn_timeout(&spec.command, timeout));
        }

        // Vendor-CLI runners read the prompt off stdin then close.
        // Write failure (EPIPE) must surface as spawn failure or
        // the API reports success with no prompt delivered.
        if let Some(payload) = spec.stdin_payload.as_deref() {
            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                if let Err(e) = stdin.write_all(payload.as_bytes()).await {
                    drop(stdin);
                    self.kill_unrecorded_child(&mut child).await;
                    self.record_spawn_failed(&id).await;
                    return Err(LifecycleError::SpawnFailed(format!(
                        "{}: stdin write failed: {e}",
                        spec.command
                    )));
                }
                drop(stdin);
            }
        }

        // Relay stdout lines to the plan bus when wired.
        if let (Some(bus), Some(plan_id), Some(stdout)) =
            (self.bus.clone(), spec.plan_id.as_ref(), child.stdout.take())
        {
            tokio::spawn(crate::stdout_relay::relay(
                stdout,
                plan_id.clone(),
                id.clone(),
                bus,
            ));
        }

        let pid = child.id().map(|p| p as i64);

        let update =
            sqlx::query("UPDATE agent_processes SET pid = ?, status = 'running' WHERE id = ?")
                .bind(pid)
                .bind(&id)
                .execute(self.pool.inner());
        if let Err(e) = timeout_query(&spec.command, timeout, started, update).await {
            self.kill_unrecorded_child(&mut child).await;
            self.record_spawn_failed(&id).await;
            return Err(e);
        }

        // Drop the child so tokio doesn't kill it on drop. The OS owns
        // the process from here; reaping happens in a follow-up loop.
        drop(child);

        Ok(AgentProcess {
            id,
            kind: spec.kind,
            command: spec.command,
            plan_id: spec.plan_id,
            task_id: spec.task_id,
            pid,
            status: ProcessStatus::Running,
            exit_code: None,
            last_heartbeat_at: None,
            started_at: now,
            ended_at: None,
        })
    }

    async fn record_spawn_failed(&self, id: &str) {
        if let Err(e) =
            sqlx::query("UPDATE agent_processes SET status = 'failed', ended_at = ? WHERE id = ?")
                .bind(Utc::now().to_rfc3339())
                .bind(id)
                .execute(self.pool.inner())
                .await
        {
            warn!(process_id = id, error = %e, "failed to record spawn failure");
        }
    }

    async fn kill_unrecorded_child(&self, child: &mut tokio::process::Child) {
        if let Err(e) = child.kill().await {
            warn!(error = %e, "failed to kill unrecorded child after spawn timeout");
        }
    }

    /// Touch `last_heartbeat_at` for the given process.
    pub async fn heartbeat(&self, id: &str) -> Result<()> {
        let n = sqlx::query("UPDATE agent_processes SET last_heartbeat_at = ? WHERE id = ?")
            .bind(Utc::now().to_rfc3339())
            .bind(id)
            .execute(self.pool.inner())
            .await?
            .rows_affected();
        if n == 0 {
            return Err(LifecycleError::NotFound(id.to_string()));
        }
        Ok(())
    }

    /// Mark a process terminated with the given exit code and status.
    /// Used by tests and (future) the OS-watcher.
    pub async fn mark_exited(&self, id: &str, exit_code: Option<i64>, ok: bool) -> Result<()> {
        let status = if ok {
            ProcessStatus::Exited
        } else {
            ProcessStatus::Failed
        };
        let n = sqlx::query(
            "UPDATE agent_processes SET status = ?, exit_code = ?, ended_at = ? WHERE id = ?",
        )
        .bind(status.as_str())
        .bind(exit_code)
        .bind(Utc::now().to_rfc3339())
        .bind(id)
        .execute(self.pool.inner())
        .await?
        .rows_affected();
        if n == 0 {
            return Err(LifecycleError::NotFound(id.to_string()));
        }
        Ok(())
    }

    /// Fetch one row by id.
    pub async fn get(&self, id: &str) -> Result<AgentProcess> {
        let row = sqlx::query_as::<_, ProcessRow>(
            "SELECT id, kind, command, plan_id, task_id, pid, status, exit_code, \
             last_heartbeat_at, started_at, ended_at FROM agent_processes \
             WHERE id = ? LIMIT 1",
        )
        .bind(id)
        .fetch_optional(self.pool.inner())
        .await?;
        row.ok_or_else(|| LifecycleError::NotFound(id.to_string()))?
            .try_into()
    }
}

#[derive(sqlx::FromRow)]
struct ProcessRow {
    id: String,
    kind: String,
    command: String,
    plan_id: Option<String>,
    task_id: Option<String>,
    pid: Option<i64>,
    status: String,
    exit_code: Option<i64>,
    last_heartbeat_at: Option<String>,
    started_at: String,
    ended_at: Option<String>,
}

impl TryFrom<ProcessRow> for AgentProcess {
    type Error = LifecycleError;
    fn try_from(r: ProcessRow) -> Result<Self> {
        Ok(AgentProcess {
            id: r.id,
            kind: r.kind,
            command: r.command,
            plan_id: r.plan_id,
            task_id: r.task_id,
            pid: r.pid,
            status: ProcessStatus::parse(&r.status).ok_or_else(|| {
                LifecycleError::InvalidStatus {
                    field: "status",
                    value: r.status.clone(),
                }
            })?,
            exit_code: r.exit_code,
            last_heartbeat_at: parse_ts_opt("last_heartbeat_at", r.last_heartbeat_at.as_deref())?,
            started_at: parse_ts("started_at", &r.started_at)?,
            ended_at: parse_ts_opt("ended_at", r.ended_at.as_deref())?,
        })
    }
}
