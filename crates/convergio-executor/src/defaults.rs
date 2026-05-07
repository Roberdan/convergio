//! Defaults the executor falls back on when a task does not opt
//! into the runner-based dispatch path.
//!
//! Split from `executor.rs` to keep the dispatcher under the
//! 300-line cap. ADR-0027 / ADR-0034.

use convergio_runner::{PermissionProfile, RunnerKind};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

impl RunnerDefaults {
    /// Read defaults from the environment, falling back to
    /// [`RunnerDefaults::default`] for any value not provided.
    ///
    /// Recognised variables:
    /// - `CONVERGIO_RUNNER_DEFAULT` — wire format `<vendor>:<model>`.
    ///   Examples: `claude:sonnet`, `claude:opus`, `copilot:gpt-5.2`,
    ///   `copilot:claude-opus`. Invalid strings log a warning and the
    ///   compiled-in default is used.
    /// - `CONVERGIO_RUNNER_PROFILE` — `standard` (default),
    ///   `restricted`, `unrestricted`.
    /// - `CONVERGIO_DAEMON_URL` — base URL the spawned agent calls
    ///   back to; defaults to `http://127.0.0.1:8420`.
    pub fn from_env() -> Self {
        let mut out = Self::default();
        if let Ok(raw) = std::env::var("CONVERGIO_RUNNER_DEFAULT") {
            match RunnerKind::from_str(raw.trim()) {
                Ok(k) => out.kind = k,
                Err(e) => tracing::warn!(
                    raw = %raw,
                    error = %e,
                    "CONVERGIO_RUNNER_DEFAULT not parseable; using compiled-in default"
                ),
            }
        }
        if let Ok(raw) = std::env::var("CONVERGIO_RUNNER_PROFILE") {
            match PermissionProfile::from_str(raw.trim()) {
                Ok(p) => out.profile = p,
                Err(e) => tracing::warn!(
                    raw = %raw,
                    error = %e,
                    "CONVERGIO_RUNNER_PROFILE not parseable; using compiled-in default"
                ),
            }
        }
        if let Ok(url) = std::env::var("CONVERGIO_DAEMON_URL") {
            if !url.trim().is_empty() {
                out.daemon_url = url;
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Env vars are process-global; serialise the cases that touch
    // them so they don't race when `cargo test` parallelises.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn clear_env() {
        for k in [
            "CONVERGIO_RUNNER_DEFAULT",
            "CONVERGIO_RUNNER_PROFILE",
            "CONVERGIO_DAEMON_URL",
        ] {
            std::env::remove_var(k);
        }
    }

    #[test]
    fn from_env_falls_back_to_default_when_unset() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        let d = RunnerDefaults::from_env();
        assert_eq!(d.kind.to_string(), "claude:sonnet");
        assert!(matches!(d.profile, PermissionProfile::Standard));
    }

    #[test]
    fn from_env_picks_copilot_when_requested() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        std::env::set_var("CONVERGIO_RUNNER_DEFAULT", "copilot:gpt-5.2");
        let d = RunnerDefaults::from_env();
        clear_env();
        assert_eq!(d.kind.to_string(), "copilot:gpt-5.2");
    }

    #[test]
    fn from_env_invalid_runner_keeps_default() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        std::env::set_var("CONVERGIO_RUNNER_DEFAULT", "garbage-no-colon");
        let d = RunnerDefaults::from_env();
        clear_env();
        assert_eq!(d.kind.to_string(), "claude:sonnet");
    }

    #[test]
    fn from_env_overrides_daemon_url() {
        let _g = ENV_LOCK.lock().unwrap();
        clear_env();
        std::env::set_var("CONVERGIO_DAEMON_URL", "http://10.0.0.1:9000");
        let d = RunnerDefaults::from_env();
        clear_env();
        assert_eq!(d.daemon_url, "http://10.0.0.1:9000");
    }
}
