//! Heartbeat sidecar for runner-spawned agents.
//!
//! Vendor CLIs in non-interactive mode (`gh copilot --yolo`,
//! `claude -p`) do not call `cvg task heartbeat <id>` themselves, no
//! matter what the prompt says. Without an external heartbeat
//! `tasks.last_heartbeat_at` stays NULL and the reaper (correctly)
//! flips the row back to `pending` after its timeout window — the
//! dispatcher then re-spawns and the system accumulates 100+ zombie
//! children in a few minutes (real incident on the first 51-task
//! run). (There is no `cvg agent heartbeat` subcommand; agent-side
//! heartbeats go through `cvg task heartbeat` or
//! `POST /v1/tasks/:id/heartbeat`.)
//!
//! For every runner spawn the executor starts a tokio task here:
//! every 60s while `kill -0 <pid>` succeeds the task ticks
//! `tasks().heartbeat(task_id)`. When the child exits and the task
//! is already in a terminal state, the sidecar best-effort removes
//! the worktree (idempotent, safe to skip on errors).

use crate::worktree;
use convergio_durability::{Durability, TaskStatus};
use convergio_lifecycle::watcher::is_alive;
use std::path::PathBuf;
use std::time::Duration;
use tracing::debug;

/// Tick `tasks.last_heartbeat_at` every minute until the child
/// process exits. On exit, sweep the worktree if the task is
/// already in a terminal state.
pub fn spawn(durability: Durability, repo_path: Option<PathBuf>, task_id: String, pid: i64) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(60));
        ticker.tick().await; // first tick fires immediately; consume
        loop {
            ticker.tick().await;
            if !is_alive(pid) {
                break;
            }
            if let Err(e) = durability.tasks().heartbeat(&task_id).await {
                debug!(task_id, error = %e, "heartbeat sidecar: tick failed");
            }
        }
        // Process exited. Tidy the worktree only when the task is
        // in a terminal state — if the agent crashed mid-edit we
        // leave the worktree alone so the operator can inspect it.
        if let Some(repo) = repo_path {
            if let Ok(t) = durability.tasks().get(&task_id).await {
                if matches!(
                    t.status,
                    TaskStatus::Done | TaskStatus::Failed | TaskStatus::Submitted
                ) {
                    worktree::cleanup(&repo, &task_id);
                }
            }
        }
    });
}
