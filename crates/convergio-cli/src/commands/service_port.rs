//! Port-management helpers for `cvg service stop` (incident
//! 2026-05-19). The service manager (launchd/systemd) only knows
//! about processes IT bootstrapped — orphan PIDs launched outside
//! the manager survive `bootout` silently. These helpers verify the
//! daemon port is released after the manager call, and kill the
//! holder if not.

use std::process::Command;
use std::time::{Duration, Instant};

/// Default daemon TCP port, mirrored from `convergio-cli`'s
/// `CONVERGIO_URL` default.
pub(super) const DEFAULT_DAEMON_PORT: u16 = 8420;

/// Extract the daemon TCP port from `CONVERGIO_URL`, falling back to
/// [`DEFAULT_DAEMON_PORT`]. Permissive parser — anything that doesn't
/// look like `scheme://host:port[/path]` returns the default rather
/// than erroring out, because `cvg service stop` must succeed even
/// when the env is misconfigured.
pub(super) fn daemon_port() -> u16 {
    parse_daemon_port(std::env::var("CONVERGIO_URL").ok().as_deref())
}

pub(super) fn parse_daemon_port(url: Option<&str>) -> u16 {
    let Some(url) = url else {
        return DEFAULT_DAEMON_PORT;
    };
    let after_scheme = url.split_once("//").map(|(_, rest)| rest).unwrap_or(url);
    let host_port = after_scheme.split('/').next().unwrap_or(after_scheme);
    match host_port.rsplit_once(':') {
        Some((_, port_str)) => port_str.parse::<u16>().unwrap_or(DEFAULT_DAEMON_PORT),
        None => DEFAULT_DAEMON_PORT,
    }
}

/// Poll `127.0.0.1:port` until it accepts a new bind (nobody
/// listening), up to `timeout`. Returns `true` on free, `false` on
/// timeout.
pub(super) fn wait_for_port_release(port: u16, timeout: Duration) -> bool {
    poll_port(port, timeout, false)
}

/// Inverse of [`wait_for_port_release`]: poll until *someone* is
/// listening on `127.0.0.1:port`. Used by `cvg service start` to
/// verify the daemon actually came up after `launchctl kickstart` /
/// `systemctl --now`.
pub(super) fn wait_for_port_bound(port: u16, timeout: Duration) -> bool {
    poll_port(port, timeout, true)
}

fn poll_port(port: u16, timeout: Duration, want_bound: bool) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if port_in_use(port) == want_bound {
            return true;
        }
        if Instant::now() >= deadline {
            return port_in_use(port) == want_bound;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

fn port_in_use(port: u16) -> bool {
    std::net::TcpListener::bind(("127.0.0.1", port)).is_err()
}

/// Find the listening PID on `127.0.0.1:port`. Returns `None` if the
/// OS-specific lookup tool is unavailable or no listener is bound.
pub(super) fn pid_on_port(port: u16) -> Option<u32> {
    if cfg!(target_os = "macos") {
        let out = Command::new("lsof")
            .args(["-ti", &format!("tcp:{port}"), "-sTCP:LISTEN"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()?
            .trim()
            .parse::<u32>()
            .ok()
    } else {
        let out = Command::new("ss")
            .args(["-lntpH", &format!("sport = :{port}")])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let Some(idx) = line.find("pid=") else {
                continue;
            };
            let rest = &line[idx + 4..];
            let end = rest.find(',').unwrap_or(rest.len());
            if let Ok(pid) = rest[..end].parse::<u32>() {
                return Some(pid);
            }
        }
        None
    }
}

pub(super) fn kill_pid(pid: u32, signal: &str) -> bool {
    Command::new("kill")
        .arg(signal)
        .arg(pid.to_string())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_daemon_port_defaults_when_missing() {
        assert_eq!(parse_daemon_port(None), DEFAULT_DAEMON_PORT);
    }

    #[test]
    fn parse_daemon_port_picks_explicit_port() {
        assert_eq!(parse_daemon_port(Some("http://127.0.0.1:9000")), 9000);
        assert_eq!(parse_daemon_port(Some("http://localhost:8421/")), 8421);
        assert_eq!(parse_daemon_port(Some("https://host:42/v1/x")), 42);
    }

    #[test]
    fn parse_daemon_port_falls_back_on_garbage() {
        assert_eq!(parse_daemon_port(Some("garbage")), DEFAULT_DAEMON_PORT);
        assert_eq!(
            parse_daemon_port(Some("http://host:not-a-number")),
            DEFAULT_DAEMON_PORT
        );
        assert_eq!(parse_daemon_port(Some("")), DEFAULT_DAEMON_PORT);
    }
}
