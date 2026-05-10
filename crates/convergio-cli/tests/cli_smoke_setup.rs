//! CLI smoke tests for `cvg setup agent <host>` — split from
//! `cli_smoke.rs` to keep both files under the 300-line cap
//! (CONSTITUTION § 13).
//!
//! Tests here verify the host-specific output of `cvg setup agent`,
//! particularly the Claude Code extras shipped by Wave 0b
//! (.claude/settings.json hook template + cvg-attach skill bundle).

use assert_cmd::Command;
use predicates::prelude::*;

fn cvg() -> Command {
    Command::cargo_bin("cvg").expect("cvg binary built")
}

#[test]
fn setup_agent_claude_emits_skill_and_settings() {
    let home = tempfile::tempdir().expect("temp home");
    cvg()
        .env("HOME", home.path())
        .args(["setup", "agent", "claude"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Claude Code extras"));
    let dir = home.path().join(".convergio/adapters/claude");
    assert!(dir.join("settings.json").is_file());
    assert!(dir.join("skill-cvg-attach/SKILL.md").is_file());
    assert!(dir.join("skill-cvg-attach/cvg-attach.sh").is_file());
    let settings = std::fs::read_to_string(dir.join("settings.json")).unwrap();
    assert!(settings.contains("SessionStart"));
    assert!(settings.contains("cvg-attach.sh"));
}

#[test]
fn setup_agent_copilot_does_not_emit_claude_extras() {
    let home = tempfile::tempdir().expect("temp home");
    cvg()
        .env("HOME", home.path())
        .args(["setup", "agent", "copilot-local"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Adapter snippets created")
                .and(predicate::str::contains("Claude Code extras").not()),
        );
    let dir = home.path().join(".convergio/adapters/copilot-local");
    assert!(!dir.join("settings.json").exists());
    assert!(!dir.join("skill-cvg-attach").exists());
}

/// `prompt.txt` for every host must contain the Step 0 bootstrap so
/// the session registers itself in the local Convergio agent registry
/// before doing anything else. Without this, peer agents cannot see
/// the session and the daemon cannot detect liveness.
#[test]
fn setup_agent_prompt_txt_contains_register_and_poll_for_every_host() {
    let cases: &[(&str, &str)] = &[
        ("claude", "claude-code-${USER}"),
        ("copilot-local", "copilot-local-${USER}-${PID}"),
        ("copilot-cloud", "copilot-cloud-${REPO_FULL_NAME}-${RUN_ID}"),
        ("cursor", "cursor-${USER}-${WORKSPACE}"),
        ("cline", "cline-${USER}"),
        ("continue", "continue-${USER}"),
        ("qwen", "qwen-${USER}"),
        ("shell", "shell-${USER}-${PPID}"),
    ];
    for (host, agent_id) in cases {
        let home = tempfile::tempdir().expect("temp home");
        cvg()
            .env("HOME", home.path())
            .args(["setup", "agent", host])
            .assert()
            .success();
        let prompt_path = home
            .path()
            .join(".convergio/adapters")
            .join(host)
            .join("prompt.txt");
        let body = std::fs::read_to_string(&prompt_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", prompt_path.display()));
        assert!(
            body.starts_with(&format!("# Convergio adapter — {host}")),
            "host {host} prompt.txt title is wrong: {:?}",
            body.lines().next()
        );
        assert!(
            body.contains("## Step 0 — register your session"),
            "host {host} prompt.txt is missing the Step 0 header"
        );
        assert!(
            body.contains(agent_id),
            "host {host} prompt.txt is missing agent_id placeholder {agent_id}"
        );
        assert!(
            body.contains("/v1/agent-registry/agents"),
            "host {host} prompt.txt is missing the registry endpoint"
        );
        assert!(
            body.contains("cvg session register-and-poll"),
            "host {host} prompt.txt should reference cvg session register-and-poll"
        );
    }
}

#[test]
fn setup_agent_bootstraps_step0_into_existing_prompt_txt() {
    let home = tempfile::tempdir().expect("temp home");
    let host = "cursor";
    let dir = home.path().join(".convergio/adapters").join(host);
    std::fs::create_dir_all(&dir).expect("create adapter dir");

    let legacy = format!("# Convergio adapter — {host}\n\n## Working loop\n\nLegacy body kept.\n");
    std::fs::write(dir.join("prompt.txt"), legacy).expect("write legacy prompt");

    cvg()
        .env("HOME", home.path())
        .args(["setup", "agent", host])
        .assert()
        .success();

    let body = std::fs::read_to_string(dir.join("prompt.txt")).expect("read patched prompt");
    assert!(
        body.starts_with(&format!("# Convergio adapter — {host}")),
        "title should be preserved"
    );
    assert!(body.contains("## Step 0 — register your session"));
    assert!(body.contains("cvg session register-and-poll"));
    assert!(
        body.contains("Legacy body kept."),
        "existing content must not be discarded"
    );
}
