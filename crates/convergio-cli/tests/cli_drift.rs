//! End-to-end test for the CLI/daemon version drift warning (P1-2).
//!
//! Spins up a tiny TCP-level fake `/v1/health` that returns a
//! deliberately mismatched `running_version`, then drives the real
//! `cvg` binary at it and asserts the warning hits stderr (not stdout)
//! while the subcommand still completes its own behavior.
//!
//! A second test verifies `CONVERGIO_NO_DRIFT_WARN=1` suppresses the
//! warning even on mismatch.

use assert_cmd::Command;
use predicates::prelude::*;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// Body shape the daemon's real `/v1/health` returns. We hard-code a
/// version that cannot match any real `CARGO_PKG_VERSION` (v0.0.0-test)
/// so the comparison is always a mismatch and the test stays stable
/// across releases.
const FAKE_DAEMON_VERSION: &str = "0.0.0-test-fake";

fn cvg() -> Command {
    Command::cargo_bin("cvg").expect("cvg binary built")
}

/// Spawn a blocking single-thread mini-server on a random local port
/// that responds to any GET with the supplied JSON body. Returns the
/// `http://127.0.0.1:<port>` URL once the listener is ready.
fn spawn_fake_daemon(daemon_version: &'static str) -> String {
    use std::io::{Read, Write};
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let url = format!("http://{addr}");

    let (ready_tx, ready_rx) = mpsc::channel::<()>();
    thread::spawn(move || {
        ready_tx.send(()).ok();
        // Serve a few connections then exit. The CLI under test only
        // makes one health probe per invocation.
        for _ in 0..8 {
            let Ok((mut sock, _)) = listener.accept() else {
                return;
            };
            sock.set_read_timeout(Some(Duration::from_millis(500))).ok();
            let mut buf = [0u8; 2048];
            let _ = sock.read(&mut buf);
            let body = format!(
                "{{\"ok\":true,\"service\":\"convergio\",\"version\":\"{daemon_version}\",\"running_version\":\"{daemon_version}\",\"expected_version\":null,\"drift\":false}}"
            );
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body,
            );
            let _ = sock.write_all(resp.as_bytes());
            let _ = sock.flush();
        }
    });
    ready_rx.recv().expect("server ready");
    url
}

#[test]
fn drift_warning_lands_on_stderr_when_daemon_version_mismatches() {
    let url = spawn_fake_daemon(FAKE_DAEMON_VERSION);
    cvg()
        .args(["--lang", "en", "--url", &url, "health"])
        .env_remove("CONVERGIO_NO_DRIFT_WARN")
        .assert()
        .success()
        .stderr(predicate::str::contains("convergio CLI is v"))
        .stderr(predicate::str::contains(FAKE_DAEMON_VERSION))
        .stderr(predicate::str::contains("cvg service restart"))
        .stderr(predicate::str::contains("CONVERGIO_NO_DRIFT_WARN"));
}

#[test]
fn drift_warning_suppressed_by_env_even_on_mismatch() {
    let url = spawn_fake_daemon(FAKE_DAEMON_VERSION);
    cvg()
        .args(["--lang", "en", "--url", &url, "health"])
        .env("CONVERGIO_NO_DRIFT_WARN", "1")
        .assert()
        .success()
        .stderr(predicate::str::contains("convergio CLI is v").not())
        .stderr(predicate::str::contains("cvg service restart").not());
}

#[test]
fn drift_warning_silent_when_daemon_unreachable() {
    // Port 1 is always closed on darwin/linux user accounts. The
    // health subcommand itself fails (that's its job), but the drift
    // probe must NOT add its own warning lines on top.
    cvg()
        .args(["--lang", "en", "--url", "http://127.0.0.1:1", "health"])
        .env_remove("CONVERGIO_NO_DRIFT_WARN")
        .assert()
        .failure()
        .stderr(predicate::str::contains("convergio CLI is v").not())
        .stderr(predicate::str::contains("cvg service restart").not());
}

#[test]
fn drift_check_skipped_for_setup_subcommand() {
    let home = tempfile::tempdir().expect("temp home");
    let url = spawn_fake_daemon(FAKE_DAEMON_VERSION);
    cvg()
        .env("HOME", home.path())
        .env_remove("CONVERGIO_NO_DRIFT_WARN")
        .args(["--lang", "en", "--url", &url, "setup"])
        .assert()
        .success()
        .stderr(predicate::str::contains("convergio CLI is v").not());
}
