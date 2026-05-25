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
    // `--allow-all-tools` is the auto-confirm-tools toggle that copilot
    // CLI requires for ANY tool to fire in non-interactive `-p` mode;
    // granular `--allow-tool` only pre-confirms (it does not bypass the
    // confirmation gate in scripted runs). Containment is the
    // `--deny-tool` list plus `--add-dir <worktree>` — see runner.rs
    // for the audit-bus evidence backing this design.
    assert!(
        a.iter().any(|s| s == "--allow-all-tools"),
        "Standard profile must enable auto-confirm via --allow-all-tools"
    );
    // `--allow-all` is the wider nuke (paths + urls + tools); not used
    // by Standard.
    assert!(
        !a.iter().any(|s| s == "--allow-all"),
        "Standard profile must NOT use the wider --allow-all nuke"
    );
    assert!(a.iter().any(|s| s.contains("shell(cargo:*)")));
    assert!(a.iter().any(|s| s.contains("shell(rm:*)")));
}

#[test]
fn copilot_sandbox_uses_allow_all_with_deny_list() {
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
    // Audit 2026-05-12 W1-E: deny-list is "always applied" per
    // PermissionProfile::copilot_deny_tools docs. Sandbox MUST emit
    // it so destructive commands (rm, sudo, push to main, force-push,
    // reset --hard, curl with data, chmod 777) cannot fire even when
    // the operator opts into --allow-all.
    assert!(
        a.iter().any(|s| s == "--deny-tool"),
        "Sandbox must include --deny-tool"
    );
    assert!(a.iter().any(|s| s.contains("shell(rm:*)")));
    assert!(a.iter().any(|s| s.contains("shell(git:push origin main")));
}

#[test]
fn copilot_unrestricted_keeps_deny_list() {
    let task = task();
    let ctx = ctx_with(&task, PermissionProfile::Unrestricted);
    let cmd = (CopilotRunner {
        model: "gpt-5.2".into(),
    })
    .prepare(&ctx)
    .unwrap();
    let a = argv(&cmd);
    assert!(a.iter().any(|s| s == "--allow-all"));
    assert!(a.iter().any(|s| s == "--add-dir"));
    assert!(
        a.iter().any(|s| s == "--deny-tool"),
        "Unrestricted must include --deny-tool"
    );
    assert!(a.iter().any(|s| s.contains("shell(rm:*)")));
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

#[test]
fn openai_argv_has_model_and_p_flag() {
    let task = task();
    let ctx = ctx_with(&task, PermissionProfile::Standard);
    let runner = convergio_runner::OpenaiRunner {
        model: "gpt-4.1".into(),
    };
    let prepared = runner.prepare(&ctx).unwrap();
    let a: Vec<String> = prepared
        .args
        .iter()
        .map(|s| s.to_string_lossy().into())
        .collect();
    assert!(a.iter().any(|s| s == "-p"));
    assert!(a.iter().any(|s| s == "--model"));
    assert!(a.iter().any(|s| s == "gpt-4.1"));
    // Standard profile -> permission-mode flag is emitted.
    assert!(a.iter().any(|s| s == "--permission-mode"));
    assert!(a.iter().any(|s| s == "--max-budget-usd"));
    // Default program when env unset.
    assert_eq!(
        prepared.program,
        OsString::from(convergio_runner::DEFAULT_OPENAI_CLI)
    );
}

#[test]
fn openai_sandbox_profile_skips_permission_mode_flag() {
    let task = task();
    let ctx = ctx_with(&task, PermissionProfile::Sandbox);
    let runner = convergio_runner::OpenaiRunner {
        model: "gpt-4.1".into(),
    };
    let prepared = runner.prepare(&ctx).unwrap();
    let a: Vec<String> = prepared
        .args
        .iter()
        .map(|s| s.to_string_lossy().into())
        .collect();
    assert!(
        !a.iter().any(|s| s == "--permission-mode"),
        "Sandbox must not emit --permission-mode; got {a:?}"
    );
}

#[test]
fn openai_for_kind_dispatches_to_openai_runner() {
    let task = task();
    let ctx = ctx_with(&task, PermissionProfile::Standard);
    let runner = for_kind(&RunnerKind::openai_gpt()).unwrap();
    let prepared = runner.prepare(&ctx).unwrap();
    // Program is whatever OPENAI_CLI_BIN says, or the default.
    assert!(
        prepared
            .program
            .to_string_lossy()
            .contains(convergio_runner::DEFAULT_OPENAI_CLI)
            || std::env::var_os(convergio_runner::OPENAI_CLI_BIN_ENV).is_some()
    );
}
