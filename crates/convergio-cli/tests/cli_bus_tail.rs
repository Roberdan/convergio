//! P1.2 smoke tests for `cvg bus tail --follow` and `cvg bus list`.
//!
//! These are help-text shape checks (the CLI repo's convention is
//! help-string smoke; e2e SSE consumption is covered by the unit
//! tests in `commands::bus_render` plus the live demo captured in
//! the PR body). The full follow loop is intentionally not exercised
//! here — it is networked + long-running and would be flaky in CI.

use assert_cmd::Command;
use predicates::prelude::*;

fn cvg() -> Command {
    Command::cargo_bin("cvg").expect("cvg binary built")
}

#[test]
fn top_level_bus_help_lists_tail_and_list() {
    cvg()
        .args(["bus", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("tail"))
        .stdout(predicate::str::contains("list"));
}

#[test]
fn bus_tail_help_documents_follow_flag() {
    cvg()
        .args(["bus", "tail", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--follow"))
        .stdout(predicate::str::contains("--plan"))
        .stdout(predicate::str::contains("--topic"))
        .stdout(predicate::str::contains("--since"))
        .stdout(predicate::str::contains("--limit"));
}

#[test]
fn bus_list_help_documents_required_flags() {
    cvg()
        .args(["bus", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--plan"))
        .stdout(predicate::str::contains("--topic"))
        .stdout(predicate::str::contains("--since"))
        .stdout(predicate::str::contains("--limit"));
}

#[test]
fn bus_list_rejects_follow_flag() {
    // `list` is the static-dump verb; --follow only belongs on tail.
    cvg().args(["bus", "list", "--follow"]).assert().failure();
}
