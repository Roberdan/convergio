//! E2E test for `cvg session pre-stop check 1` (plan-vs-merged-PR
//! drift).
//!
//! Strategy: drop fake `gh` and `curl` shim scripts onto a tempdir
//! `PATH`. The shims read fixture files chosen via env vars so each
//! test case can dictate the merged-PR list and the daemon's task
//! lookup response. The CLI then runs `session pre-stop --output
//! json` and we assert on the structured output for the
//! `check.plan_pr_drift` row.

use assert_cmd::Command;
use std::ffi::OsString;
use std::path::Path;
use std::{env, fs};

fn cvg() -> Command {
    Command::cargo_bin("cvg").expect("cvg binary built")
}

fn path_with_bin_dir(bin_dir: &Path) -> OsString {
    let mut paths = vec![bin_dir.to_path_buf()];
    if let Some(path) = env::var_os("PATH") {
        paths.extend(env::split_paths(&path));
    }
    env::join_paths(paths).expect("valid PATH")
}

#[cfg(unix)]
fn write_executable(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, body).expect("write shim");
    let mut p = fs::metadata(path).expect("meta").permissions();
    p.set_mode(0o755);
    fs::set_permissions(path, p).expect("chmod shim");
}

/// Build a tempdir with `gh` + `curl` shims that print fixture-file
/// contents for `pr list` / task-status calls. Returns
/// `(tempdir, bin_dir, gh_fixture_path, curl_fixture_path)`.
fn shim_env(
    gh_fixture: &str,
    curl_fixture: &str,
) -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).expect("mk bin");
    let gh_fix = dir.path().join("gh.json");
    let curl_fix = dir.path().join("curl.json");
    fs::write(&gh_fix, gh_fixture).expect("gh fix");
    fs::write(&curl_fix, curl_fixture).expect("curl fix");

    #[cfg(unix)]
    {
        write_executable(
            &bin.join("gh"),
            &format!(
                "#!/bin/sh\nif [ \"$1\" = \"pr\" ] && [ \"$2\" = \"list\" ]; then\n  cat {}\nelse\n  echo '[]'\nfi\n",
                gh_fix.display()
            ),
        );
        write_executable(
            &bin.join("curl"),
            &format!("#!/bin/sh\ncat {}\n", curl_fix.display()),
        );
    }
    (dir, bin, curl_fix)
}

#[test]
#[cfg(unix)]
fn pre_stop_passes_when_no_merged_prs_reference_tasks() {
    let (_dir, bin, _curl_fix) = shim_env("[]", "{}");
    // `--force` because other registry checks (e.g. worktree.no_pr)
    // run against the real working tree and may legitimately fail.
    // We only assert the plan_pr_drift row here.
    let assert = cvg()
        .env("PATH", path_with_bin_dir(&bin))
        .args([
            "session",
            "pre-stop",
            "--agent-id",
            "claude-code-test",
            "--output",
            "json",
            "--force",
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    // The plan_pr_drift check should be Pass (no findings).
    assert!(
        out.contains("\"check.plan_pr_drift\""),
        "report missing plan_pr_drift row: {out}"
    );
    let drift_block = out
        .split("\"check.plan_pr_drift\"")
        .nth(1)
        .expect("after id")
        .split("\"id\"")
        .next()
        .expect("up to next id");
    assert!(
        drift_block.contains("\"pass\""),
        "expected pass for empty merged list, got: {drift_block}"
    );
}

#[test]
#[cfg(unix)]
fn pre_stop_flags_drift_when_merged_pr_tracks_pending_task() {
    // Merged PR #42 references task 5298055b... but the daemon says
    // that task is still `pending` — that's the drift signal.
    let gh = r#"[{"number":42,"title":"feat(x): close 5298055b","body":"Tracks: 5298055b-9e2b-4822-a2bc-9cb1aa3e28ea"}]"#;
    let curl = r#"{"id":"5298055b-9e2b-4822-a2bc-9cb1aa3e28ea","status":"pending"}"#;
    let (_dir, bin, _curl_fix) = shim_env(gh, curl);
    let assert = cvg()
        .env("PATH", path_with_bin_dir(&bin))
        .args([
            "session",
            "pre-stop",
            "--agent-id",
            "claude-code-test",
            "--output",
            "json",
            "--force",
        ])
        .assert()
        .success(); // --force makes detach succeed even with findings
    let out = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(out.contains("\"check.plan_pr_drift\""), "row present");
    // Drift row should serialise the `Fail` variant with the merged PR
    // referenced by number.
    assert!(
        out.contains("\"fail\""),
        "expected at least one fail, got: {out}"
    );
    assert!(
        out.contains("PR #42") || out.contains("#42"),
        "expected merged PR # in finding, got: {out}"
    );
}

#[test]
#[cfg(unix)]
fn pre_stop_passes_when_merged_pr_task_already_done() {
    // Same shape as the failing case, but the task state is `done`,
    // so there is no drift.
    let gh =
        r#"[{"number":99,"title":"feat","body":"Tracks: 5298055b-9e2b-4822-a2bc-9cb1aa3e28ea"}]"#;
    let curl = r#"{"id":"5298055b-9e2b-4822-a2bc-9cb1aa3e28ea","status":"done"}"#;
    let (_dir, bin, _curl_fix) = shim_env(gh, curl);
    let assert = cvg()
        .env("PATH", path_with_bin_dir(&bin))
        .args([
            "session",
            "pre-stop",
            "--agent-id",
            "claude-code-test",
            "--output",
            "json",
            "--force",
        ])
        .assert()
        .success();
    let out = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    let drift_block = out
        .split("\"check.plan_pr_drift\"")
        .nth(1)
        .expect("after id")
        .split("\"id\"")
        .next()
        .expect("up to next id");
    assert!(
        drift_block.contains("\"pass\""),
        "expected pass when task is done, got: {drift_block}"
    );
}
