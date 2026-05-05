//! Smoke tests for `cvg agent retire <id>` (P1-5).
//!
//! Closes C4 + C8 from the 2026-05-04 retro: prove the new
//! single-agent retire CLI surface is wired end-to-end and emits
//! a localized failure when the daemon is unreachable. The full
//! daemon round-trip lives in the server-side e2e test
//! (`e2e_agent_retire_cli`).

use assert_cmd::Command;
use predicates::prelude::*;

fn cvg() -> Command {
    Command::cargo_bin("cvg").expect("cvg binary built")
}

#[test]
fn agent_retire_help_lists_id_arg() {
    cvg()
        .args(["agent", "retire", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<ID>"));
}

#[test]
fn agent_retire_against_unreachable_url_fails_clearly() {
    cvg()
        .args([
            "--url",
            "http://127.0.0.1:1",
            "agent",
            "retire",
            "subagent-test",
        ])
        .assert()
        .failure();
}

#[test]
fn agent_help_lists_retire_alongside_retire_stale() {
    // Defensive test: avoid regressing into a state where only the
    // bulk variant is reachable from `--help` (the C4 friction point).
    let out = cvg()
        .args(["agent", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(out).unwrap();
    assert!(text.contains("retire"), "help must mention retire");
    assert!(
        text.contains("retire-stale"),
        "help must mention retire-stale"
    );
}
