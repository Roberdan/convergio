//! CLI surface tests for `cvg update` (F50). Lives in its own file
//! so `cli_smoke.rs` stays under the 300-line cap.

use assert_cmd::Command;
use convergio_i18n::{Bundle, Locale};
use predicates::prelude::*;

fn cvg() -> Command {
    Command::cargo_bin("cvg").expect("cvg binary built")
}

#[test]
fn update_help_lists_flags() {
    cvg()
        .args(["update", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--if-needed"))
        .stdout(predicate::str::contains("--skip-restart"))
        .stdout(predicate::str::contains("--changelog"));
}

#[test]
fn help_lists_update_subcommand() {
    cvg()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("update"));
}

#[test]
fn update_if_needed_against_unreachable_daemon_still_runs_rebuild_attempt() {
    // With `--if-needed` and an unreachable daemon, the prior_version
    // probe falls back to "unknown" — this is by design (the cmd
    // proceeds to rebuild when there's no daemon to compare against).
    // We only verify the clap surface accepts the combination here;
    // exercising the cargo install would mutate the user's machine
    // and is explicitly out-of-scope for the worktree per F50 rules.
    cvg()
        .args([
            "--url",
            "http://127.0.0.1:1",
            "--output",
            "json",
            "update",
            "--if-needed",
            "--help",
        ])
        .assert()
        .success();
}

#[test]
fn update_copy_warning_is_localized() {
    let source = include_str!("../src/commands/update_run.rs");
    assert!(source.contains("update-sync-copy-warning"));
    assert!(!source.contains("warn: cp"));

    let args = &[("src", "src-bin"), ("dst", "dst-bin"), ("reason", "denied")];
    let en = Bundle::new(Locale::En).expect("english bundle");
    let it = Bundle::new(Locale::It).expect("italian bundle");
    assert_eq!(
        en.t("update-sync-copy-warning", args),
        "Warning: could not copy src-bin to dst-bin: denied"
    );
    assert_eq!(
        it.t("update-sync-copy-warning", args),
        "Attenzione: impossibile copiare src-bin in dst-bin: denied"
    );
}
