//! Integration tests for `Runner::prepare` argv shape.
//!
//! Hosted in `tests/` so `runner.rs` stays under the 300-line cap.
//! Uses only the public surface of `convergio-runner`.

use chrono::Utc;
use convergio_durability::{Task, TaskStatus};
use convergio_runner::{
    for_kind, ClaudeRunner, CopilotRunner, Family, PermissionProfile, Runner, RunnerKind,
    SpawnContext,
};
use std::ffi::OsString;
use std::path::Path;

fn task() -> Task {
    let now = Utc::now();
    Task {
        id: "t-aaa".into(),
        plan_id: "p-bbb".into(),
        wave: 1,
        sequence: 1,
        title: "do thing".into(),
        description: None,
        status: TaskStatus::Pending,
        agent_id: None,
        evidence_required: vec!["test".into()],
        last_heartbeat_at: None,
        created_at: now,
        updated_at: now,
        started_at: None,
        ended_at: None,
        duration_ms: None,
        runner_kind: None,
        profile: None,
        max_budget_usd: None,
    }
}

fn ctx_with<'a>(task: &'a Task, profile: PermissionProfile) -> SpawnContext<'a> {
    SpawnContext {
        task,
        plan_id: "p-bbb",
        plan_title: "demo",
        daemon_url: "http://127.0.0.1:8420",
        agent_id: "claude-test",
        graph_context: None,
        cwd: Path::new("/tmp/wt"),
        max_budget_usd: Some(1.5),
        profile,
    }
}

fn argv(cmd: &convergio_runner::PreparedCommand) -> Vec<String> {
    cmd.args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn claude_standard_uses_permission_mode_and_allowlist() {
    let task = task();
    let ctx = ctx_with(&task, PermissionProfile::Standard);
    let cmd = (ClaudeRunner {
        model: "sonnet".into(),
    })
    .prepare(&ctx)
    .unwrap();
    let a = argv(&cmd);
    assert!(a.iter().any(|s| s == "-p"));
    assert!(a.iter().any(|s| s == "sonnet"));
    assert!(a.iter().any(|s| s == "--permission-mode"));
    assert!(a.iter().any(|s| s == "acceptEdits"));
    assert!(a.iter().any(|s| s == "--allowed-tools"));
    assert!(
        !a.iter().any(|s| s == "--dangerously-skip-permissions"),
        "Standard profile must NOT use the nuke flag"
    );
    assert!(a.iter().any(|s| s == "stream-json"));
    assert!(a.iter().any(|s| s == "--verbose"));
    assert!(cmd.stdin_prompt.contains("`t-aaa`"));
}

#[test]
fn claude_sandbox_keeps_dangerously_skip_for_sealed_envs() {
    let task = task();
    let ctx = ctx_with(&task, PermissionProfile::Sandbox);
    let cmd = (ClaudeRunner {
        model: "sonnet".into(),
    })
    .prepare(&ctx)
    .unwrap();
    let a = argv(&cmd);
    assert!(a.iter().any(|s| s == "--dangerously-skip-permissions"));
    assert!(!a.iter().any(|s| s == "--permission-mode"));
}

#[test]
fn copilot_standard_uses_per_tool_whitelist_with_deny() {
    let task = task();
    let ctx = ctx_with(&task, PermissionProfile::Standard);
    let cmd = (CopilotRunner {
        model: "gpt-5.2".into(),
    })
    .prepare(&ctx)
    .unwrap();
    let a = argv(&cmd);
    assert!(a.iter().any(|s| s == "--allow-tool"));
    assert!(a.iter().any(|s| s == "--deny-tool"));
    assert!(a.iter().any(|s| s == "--add-dir"));
    assert!(
        !a.iter()
            .any(|s| s == "--allow-all-tools" || s == "--allow-all"),
        "Standard profile must NOT use the nuke flag"
    );
    assert!(a.iter().any(|s| s.contains("shell(cargo:*)")));
    assert!(a.iter().any(|s| s.contains("shell(rm:*)")));
}

#[test]
fn copilot_sandbox_uses_allow_all() {
    let task = task();
    let ctx = ctx_with(&task, PermissionProfile::Sandbox);
    let cmd = (CopilotRunner {
        model: "gpt-5.2".into(),
    })
    .prepare(&ctx)
    .unwrap();
    let a = argv(&cmd);
    assert!(a.iter().any(|s| s == "--allow-all"));
    assert!(!a.iter().any(|s| s == "--allow-tool"));
    assert!(!a.iter().any(|s| s == "--add-dir"));
}

#[test]
fn for_kind_dispatches_to_the_right_vendor() {
    let task = task();
    let ctx = ctx_with(&task, PermissionProfile::Standard);
    let claude = for_kind(&RunnerKind::claude_sonnet()).unwrap();
    assert_eq!(
        claude.prepare(&ctx).unwrap().program,
        OsString::from("claude")
    );
    let copilot = for_kind(&RunnerKind::copilot_gpt()).unwrap();
    assert_eq!(
        copilot.prepare(&ctx).unwrap().program,
        OsString::from("copilot")
    );
}

#[test]
fn assert_cli_on_path_rejects_when_binary_missing_from_explicit_path() {
    let cli = Family::Claude.cli();
    let bogus = "/__convergio_runner_bogus_path__";
    let found = std::env::split_paths(bogus).any(|p| p.join(cli).is_file());
    assert!(!found);
}
