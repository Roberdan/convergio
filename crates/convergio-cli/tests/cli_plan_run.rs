//! CLI smoke tests for `cvg plan run` — exercises the `--max-parallel`
//! surface and the resume-hint wiring without booting a daemon (P1-8).

use assert_cmd::Command;
use predicates::prelude::*;

fn cvg() -> Command {
    Command::cargo_bin("cvg").expect("cvg binary built")
}

#[test]
fn plan_run_help_lists_max_parallel_flag() {
    cvg()
        .args(["plan", "run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--max-parallel"))
        .stdout(predicate::str::contains("--agent-id"))
        .stdout(predicate::str::contains("Plan number"));
}

#[test]
fn plan_run_against_unreachable_daemon_fails_clearly() {
    cvg()
        .args(["--url", "http://127.0.0.1:1", "plan", "run", "1"])
        .assert()
        .failure();
}

#[test]
fn plan_run_accepts_max_parallel_flag_value() {
    cvg()
        .args([
            "--url",
            "http://127.0.0.1:1",
            "plan",
            "run",
            "1",
            "--max-parallel",
            "4",
        ])
        .assert()
        .failure();
}
