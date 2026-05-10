//! Single-instance daemon lock via `~/.convergio/daemon.pid`.
//!
//! Extracted from `main.rs` to keep the entry point under the 300-line
//! crate-wide cap and to give the lock its own test surface.
//!
//! Two daemons sharing the same SQLite file race their executor ticks
//! and dispatch each task `n_daemons × MAX_PARALLEL` times, blowing
//! the cap (incident 2026-05-10: 8 tasks dispatched against
//! `CONVERGIO_EXECUTOR_MAX_PARALLEL=2` because launchctl + a manually
//! started daemon were both ticking against `~/.convergio/v3/state.db`).

use std::path::Path;
use std::process::{Command, Stdio};

/// Refuse to start when another live daemon already owns the PID file.
///
/// Stale PID files (process gone) are silently overwritten; same-PID
/// rewrites (re-exec from inside the same daemon) are no-ops. The lock
/// path is `<home>/.convergio/daemon.pid`.
pub fn claim() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let dir = Path::new(&home).join(".convergio");
    std::fs::create_dir_all(&dir)?;
    let pid_file = dir.join("daemon.pid");
    let my_pid = std::process::id();

    if let Ok(prev) = std::fs::read_to_string(&pid_file) {
        if let Ok(prev_pid) = prev.trim().parse::<i32>() {
            if prev_pid > 0 && prev_pid != my_pid as i32 && pid_alive(prev_pid) {
                return Err(format!(
                    "refusing to start: another convergio daemon is already running (pid {prev_pid}). \
                     Stop it first (`launchctl stop com.convergio.v3`, `kill {prev_pid}`, or remove \
                     {} if you are sure the previous daemon is gone) before starting a new one.",
                    pid_file.display()
                )
                .into());
            }
        }
    }

    std::fs::write(&pid_file, my_pid.to_string())?;
    Ok(())
}

/// Returns `true` if `pid` is a live process this user can signal.
///
/// Implemented via `kill -0 <pid>` rather than `libc::kill` so the
/// crate keeps `#![forbid(unsafe_code)]`. `kill -0` is POSIX and
/// only checks for existence — no signal is delivered.
pub fn pid_alive(pid: i32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pid_alive_returns_true_for_self() {
        let me = std::process::id() as i32;
        assert!(pid_alive(me), "kill -0 against own pid must succeed");
    }

    #[test]
    fn pid_alive_returns_false_for_implausible_pid() {
        // i32::MAX is well above the kernel's pid_max on every
        // platform we ship to (Linux default 4194304, macOS 99999),
        // so it is guaranteed to be unallocated.
        assert!(!pid_alive(i32::MAX));
    }
}
