//! `cvg pr merge` — CLI surface integration tests. The orchestration
//! body is unit-tested in `pr_merge*.rs`; here we exercise the
//! clap-parsed surface only (no real gh / git spawned).

use assert_cmd::Command;
use predicates::prelude::*;

fn cvg() -> Command {
    Command::cargo_bin("cvg").expect("cvg binary built")
}

#[test]
fn pr_help_lists_merge_subcommand() {
    cvg()
        .args(["pr", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("merge"));
}

#[test]
fn pr_merge_help_lists_all_advertised_flags() {
    let assert = cvg().args(["pr", "merge", "--help"]).assert().success();
    assert
        .stdout(predicate::str::contains("retire-agent"))
        .stdout(predicate::str::contains("dry-run"))
        .stdout(predicate::str::contains("no-cleanup"));
}

#[test]
fn pr_merge_requires_pr_number() {
    cvg().args(["pr", "merge"]).assert().failure();
}

#[test]
fn pr_merge_rejects_non_numeric_pr() {
    cvg()
        .args(["pr", "merge", "not-a-number"])
        .assert()
        .failure();
}
