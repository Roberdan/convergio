//! Domain types for Layer 3.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Process lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessStatus {
    /// Spawned but PID not yet captured.
    Starting,
    /// PID captured, process is alive.
    Running,
    /// Process exited cleanly.
    Exited,
    /// Process exited non-zero or could not start.
    Failed,
}

impl ProcessStatus {
    /// String tag persisted in the DB.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Exited => "exited",
            Self::Failed => "failed",
        }
    }

    /// Parse from the DB.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "starting" => Some(Self::Starting),
            "running" => Some(Self::Running),
            "exited" => Some(Self::Exited),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

/// A persisted process row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProcess {
    /// UUID v4.
    pub id: String,
    /// Logical kind (`claude-code`, `shell`, `python`, ...).
    pub kind: String,
    /// argv0 — used for diagnostics, not re-spawn.
    pub command: String,
    /// Plan this process is helping with, if any.
    pub plan_id: Option<String>,
    /// Task this process is holding, if any.
    pub task_id: Option<String>,
    /// OS pid; `None` while `Starting`.
    pub pid: Option<i64>,
    /// Status.
    pub status: ProcessStatus,
    /// Exit code if `Exited`/`Failed`.
    pub exit_code: Option<i64>,
    /// Last heartbeat timestamp.
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    /// Spawn timestamp.
    pub started_at: DateTime<Utc>,
    /// End timestamp if known.
    pub ended_at: Option<DateTime<Utc>>,
}

/// Input for [`crate::Supervisor::spawn`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnSpec {
    /// Logical kind tag stored in the row.
    pub kind: String,
    /// argv0.
    pub command: String,
    /// argv[1..].
    #[serde(default)]
    pub args: Vec<String>,
    /// Extra environment variables (`KEY=VALUE` pairs).
    #[serde(default)]
    pub env: Vec<(String, String)>,
    /// Plan id this process is associated with, if any.
    #[serde(default)]
    pub plan_id: Option<String>,
    /// Task id this process is holding, if any.
    #[serde(default)]
    pub task_id: Option<String>,
    /// Working directory the child should run in. `None` ⇒ inherit
    /// the daemon's cwd (legacy behaviour).
    #[serde(default)]
    pub cwd: Option<std::path::PathBuf>,
    /// Optional payload piped on the child's stdin and then closed.
    /// Used by vendor-CLI runners (claude, copilot, qwen) that read
    /// the prompt from stdin in non-interactive mode. `None` ⇒
    /// `Stdio::null()` (legacy behaviour, no input).
    #[serde(default)]
    pub stdin_payload: Option<String>,
}
