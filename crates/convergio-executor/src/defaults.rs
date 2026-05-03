//! Defaults the executor falls back on when a task does not opt
//! into the runner-based dispatch path.
//!
//! Split from `executor.rs` to keep the dispatcher under the
//! 300-line cap. ADR-0027 / ADR-0034.

use convergio_runner::{PermissionProfile, RunnerKind};
use serde::{Deserialize, Serialize};

/// Template the executor uses for tasks that opt out of runner-based
/// dispatch. ADR-0034 introduced per-task `runner_kind` / `profile`
/// columns; tasks that have them populated are spawned through
/// [`convergio_runner`] instead of this template. The template path
/// is kept as the legacy fallback (and for shell-only smoke tests).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnTemplate {
    /// argv0.
    pub command: String,
    /// argv[1..n] — the task id is appended after these.
    pub args: Vec<String>,
    /// Logical kind tag (passed through to `agent_processes.kind`).
    pub kind: String,
}

impl Default for SpawnTemplate {
    fn default() -> Self {
        Self {
            command: "/bin/echo".into(),
            args: vec!["task".into()],
            kind: "shell".into(),
        }
    }
}

/// Daemon-wide defaults applied when a task has no per-task
/// `runner_kind` or `profile`. Read from env at boot.
#[derive(Debug, Clone)]
pub struct RunnerDefaults {
    /// Wire format `<vendor>:<model>`. Default `claude:sonnet`.
    pub kind: RunnerKind,
    /// Default permission profile.
    pub profile: PermissionProfile,
    /// Daemon HTTP base URL the agent calls back to (for `cvg`).
    pub daemon_url: String,
}

impl Default for RunnerDefaults {
    fn default() -> Self {
        Self {
            kind: RunnerKind::claude_sonnet(),
            profile: PermissionProfile::Standard,
            daemon_url: "http://127.0.0.1:8420".into(),
        }
    }
}
