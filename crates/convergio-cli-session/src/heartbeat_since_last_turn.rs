//! `cvg session heartbeat-since-last-turn` — fire-and-forget heartbeat
//! from the Claude Code `PreToolUse` hook (P1-3).
//!
//! Goal: surface long-running sessions as "active" in `cvg agent list`
//! without blocking tool execution. The hook is on the hot path of
//! every Bash / Edit / Write call, so this command must (a) cost
//! ~nothing when called more often than the threshold, and (b) never
//! propagate an error to the caller.
//!
//! Mechanism: a per-pid timestamp file under
//! `~/.convergio/state/sessions/<pid>.last-hb` records the last
//! successful heartbeat in seconds-since-epoch. If the file is
//! younger than [`HEARTBEAT_INTERVAL`], the command exits silently.
//! If older or missing, it POSTs `/v1/agent-registry/agents/:id/heartbeat`
//! and rewrites the file.
//!
//! Output: nothing on success. On the FIRST call (file missing) a
//! single stderr line `convergio: heartbeating <agent-id> every ~5
//! min` confirms the hook is wired so operators don't wonder why
//! `agent list` is suddenly fresh.

use crate::Client;
use anyhow::Result;
use serde_json::json;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Minimum interval between heartbeats. Smaller values waste daemon
/// CPU; larger values risk a session looking stale in `cvg agent
/// list`. 5 minutes is the value the v3 reaper grace also defaults
/// to, so an active session never crosses the reap threshold.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5 * 60);

/// Entry point. Always returns `Ok(())` — errors are swallowed at the
/// `cvg` boundary so a transient daemon glitch never blocks a tool
/// call. The function returns `Result` only so callers can keep the
/// `?` syntax uniform with the rest of `SessionCommand`.
pub async fn run(client: &Client, agent_id: Option<String>, status: String) -> Result<()> {
    let id = resolve_agent_id(agent_id);
    let path = match timestamp_path() {
        Some(p) => p,
        None => return Ok(()),
    };
    let now = SystemTime::now();
    let first_call = !path.exists();
    if !should_heartbeat(&path, now) {
        return Ok(());
    }
    if first_call && should_show_banner() {
        eprintln!("convergio: heartbeating {id} every ~5 min");
    }
    let body = json!({"status": status});
    let post_ok = client
        .post::<_, serde_json::Value>(&format!("/v1/agent-registry/agents/{id}/heartbeat"), &body)
        .await
        .is_ok();
    // Only refresh the throttle when the POST actually succeeded;
    // otherwise repeated failures would stay hidden for one full
    // `HEARTBEAT_INTERVAL`. The outward error is still swallowed so
    // a transient daemon glitch never blocks the calling tool.
    if post_ok {
        let _ = write_timestamp(&path, now);
    }
    Ok(())
}

/// Resolve the agent id with the same precedence as the
/// `register-and-poll` command so the two stay in sync.
fn resolve_agent_id(flag: Option<String>) -> String {
    if let Some(id) = flag {
        return id;
    }
    if let Ok(id) = std::env::var("CONVERGIO_AGENT_ID") {
        if !id.is_empty() {
            return id;
        }
    }
    let user = std::env::var("USER").unwrap_or_else(|_| "anon".to_string());
    format!("claude-code-{user}")
}

/// Path to the per-pid timestamp file. `None` only when no home
/// directory can be located — in that case we silently skip the
/// heartbeat (the alternative would be to spam `/tmp` files).
fn timestamp_path() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    let pid = std::process::id();
    let mut p = PathBuf::from(home);
    p.push(".convergio");
    p.push("state");
    p.push("sessions");
    let _ = std::fs::create_dir_all(&p);
    p.push(format!("{pid}.last-hb"));
    Some(p)
}

/// Decide whether enough wall-time has elapsed since the last
/// successful heartbeat. Missing / unreadable / corrupt timestamps
/// always heartbeat — better one extra POST than a silent session.
pub(crate) fn should_heartbeat(path: &PathBuf, now: SystemTime) -> bool {
    let Ok(text) = std::fs::read_to_string(path) else {
        return true;
    };
    let Ok(secs) = text.trim().parse::<u64>() else {
        return true;
    };
    let last = UNIX_EPOCH + Duration::from_secs(secs);
    match now.duration_since(last) {
        Ok(elapsed) => elapsed >= HEARTBEAT_INTERVAL,
        Err(_) => true, // clock went backwards; refresh defensively.
    }
}

fn write_timestamp(path: &PathBuf, now: SystemTime) -> std::io::Result<()> {
    let secs = now
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    std::fs::write(path, secs.to_string())
}

/// Should the first-call "heartbeating …" banner be emitted?
///
/// Returns `true` only when stderr is a terminal. The PreToolUse
/// hook runs with piped stderr, so the hardcoded English banner is
/// suppressed in that path — addresses the P5 constitution audit
/// follow-up (2026-05-12, src/heartbeat_since_last_turn.rs:50).
/// Interactive operators still see the message when they invoke
/// `cvg session heartbeat-since-last-turn` by hand.
pub(crate) fn should_show_banner() -> bool {
    use std::io::IsTerminal;
    std::io::stderr().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Duration;
    use tempfile::tempdir;

    // Tests in a single binary share process-global env. Without
    // serialization the `HOME` writes race and the file-existence
    // assertions are flaky.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn missing_file_triggers_heartbeat() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("missing.last-hb");
        assert!(should_heartbeat(&p, SystemTime::now()));
    }

    #[test]
    fn fresh_timestamp_skips_heartbeat() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("fresh.last-hb");
        let now = SystemTime::now();
        write_timestamp(&p, now).unwrap();
        assert!(!should_heartbeat(&p, now));
    }

    #[test]
    fn stale_timestamp_triggers_heartbeat() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("stale.last-hb");
        let past = SystemTime::now() - Duration::from_secs(6 * 60);
        write_timestamp(&p, past).unwrap();
        assert!(should_heartbeat(&p, SystemTime::now()));
    }

    #[test]
    fn corrupt_timestamp_triggers_heartbeat() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("corrupt.last-hb");
        std::fs::write(&p, "not-a-number").unwrap();
        assert!(should_heartbeat(&p, SystemTime::now()));
    }

    #[test]
    fn at_threshold_triggers_heartbeat() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("threshold.last-hb");
        let past = SystemTime::now() - HEARTBEAT_INTERVAL;
        write_timestamp(&p, past).unwrap();
        assert!(should_heartbeat(&p, SystemTime::now()));
    }

    /// Regression: a failed heartbeat POST must NOT rewrite the
    /// throttle timestamp, otherwise repeated failures stay invisible
    /// for one full [`HEARTBEAT_INTERVAL`]. See the
    /// `convergio-cli-session` audit follow-up (2026-05-12).
    #[tokio::test(flavor = "current_thread")]
    #[allow(clippy::await_holding_lock)] // serializing $HOME across one cheap await is intentional
    async fn failed_post_does_not_persist_throttle_timestamp() {
        let _g = ENV_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let prev_home = std::env::var_os("HOME");
        std::env::set_var("HOME", dir.path());

        // Port 1 is reserved and unreachable; the POST resolves to an
        // immediate `Err` without external dependencies.
        let client = crate::Client::new("http://127.0.0.1:1".to_string());
        run(&client, Some("audit-test".into()), "working".into())
            .await
            .expect("run swallows the outward error");

        let path = timestamp_path().expect("HOME resolves to a path");
        let exists = path.exists();

        if let Some(h) = prev_home {
            std::env::set_var("HOME", h);
        } else {
            std::env::remove_var("HOME");
        }

        assert!(
            !exists,
            "failed heartbeat POST must not refresh the throttle timestamp"
        );
    }

    /// Regression: the hot-path PreToolUse hook never has a TTY on
    /// stderr, so the hardcoded English "heartbeating …" banner has
    /// no operator to read it. [`should_show_banner`] must return
    /// `false` whenever stderr is not a terminal, otherwise the
    /// non-i18n string ships in every hook invocation. See the
    /// `convergio-cli-session` audit follow-up (2026-05-12):
    /// src/heartbeat_since_last_turn.rs:50.
    #[test]
    fn banner_is_suppressed_when_stderr_is_not_a_tty() {
        use std::io::IsTerminal;
        // Test invariant: cargo test pipes stderr.
        assert!(
            !std::io::stderr().is_terminal(),
            "expected piped stderr under cargo test"
        );
        assert!(
            !should_show_banner(),
            "hot-path hook must not emit hardcoded English banner without a TTY"
        );
    }
}
