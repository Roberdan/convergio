//! CLI smoke tests for `cvg session register-and-poll`. These cover
//! clap wiring only; the round-trip against a real daemon lives in
//! `convergio-server/tests/e2e_session_register_and_poll.rs`, which
//! has the in-process server fixture available.

use assert_cmd::Command;
use predicates::prelude::*;

fn cvg() -> Command {
    Command::cargo_bin("cvg").expect("cvg binary built")
}

#[test]
fn session_help_lists_register_and_poll() {
    cvg()
        .args(["session", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("register-and-poll"));
}

#[test]
fn register_and_poll_help_lists_flags() {
    cvg()
        .args(["session", "register-and-poll", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--agent-id"))
        .stdout(predicate::str::contains("--capability"))
        .stdout(predicate::str::contains("--kind"))
        .stdout(predicate::str::contains("--host"))
        .stdout(predicate::str::contains("--quiet"));
}

#[test]
fn register_and_poll_against_unreachable_url_fails_clearly() {
    cvg()
        .args([
            "--url",
            "http://127.0.0.1:1",
            "session",
            "register-and-poll",
            "--agent-id",
            "smoke-only",
        ])
        .assert()
        .failure();
}
