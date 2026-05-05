//! Tests for the project-level Claude Code `SessionStart` hook
//! in `.claude/settings.json` (P2-6).
//!
//! The hook must:
//!   1. Run `cvg session register-and-poll` (existing behavior, #171).
//!   2. Also run `cvg session resume --output plain` so the agent's
//!      first turn has live state without being told to call it.
//!   3. Print both blocks behind clear marker lines so the agent
//!      sees them as two distinct sections.
//!   4. Honour `CONVERGIO_NO_AUTO_RESUME=1` as the opt-out.
//!
//! Two layers of coverage:
//!   - **Settings parse**: read the committed `.claude/settings.json`,
//!     drill into the SessionStart hook command string, assert it
//!     references both subcommands, both markers, and the env-var
//!     escape hatch.
//!   - **Smoke**: invoke a small bash harness that replaces `cvg`
//!     with stubbed echoers, run the same command shape the hook
//!     uses, and assert the captured output contains both markers
//!     and that the opt-out actually skips the resume half.

use std::path::PathBuf;
use std::process::Command;

const BOOTSTRAP_MARKER: &str = "=== Convergio session bootstrap ===";
const RESUME_MARKER: &str = "=== Convergio session resume (live state) ===";

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points at crates/convergio-cli; go up two.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above crates/convergio-cli")
        .to_path_buf()
}

fn settings_json() -> String {
    let path = workspace_root().join(".claude").join("settings.json");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn session_start_command(settings: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(settings).expect("parse settings.json");
    let entries = v
        .get("hooks")
        .and_then(|h| h.get("SessionStart"))
        .and_then(|s| s.as_array())
        .expect("hooks.SessionStart is an array");
    let mut commands = Vec::new();
    for entry in entries {
        if let Some(inner) = entry.get("hooks").and_then(|h| h.as_array()) {
            for h in inner {
                if let Some(cmd) = h.get("command").and_then(|c| c.as_str()) {
                    commands.push(cmd.to_string());
                }
            }
        }
    }
    assert!(
        !commands.is_empty(),
        "no SessionStart command found in settings.json"
    );
    // The repo only ships one SessionStart command today.
    commands.join("\n")
}

#[test]
fn session_start_hook_invokes_both_register_and_resume() {
    let settings = settings_json();
    let cmd = session_start_command(&settings);

    assert!(
        cmd.contains("session register-and-poll"),
        "SessionStart hook missing register-and-poll: {cmd}"
    );
    assert!(
        cmd.contains("session resume"),
        "SessionStart hook missing session resume: {cmd}"
    );
    assert!(
        cmd.contains("--output plain"),
        "session resume should be invoked with --output plain for hook capture: {cmd}"
    );
}

#[test]
fn session_start_hook_emits_both_markers() {
    let settings = settings_json();
    let cmd = session_start_command(&settings);
    assert!(
        cmd.contains(BOOTSTRAP_MARKER),
        "SessionStart hook missing bootstrap marker '{BOOTSTRAP_MARKER}': {cmd}"
    );
    assert!(
        cmd.contains(RESUME_MARKER),
        "SessionStart hook missing resume marker '{RESUME_MARKER}': {cmd}"
    );
}

#[test]
fn session_start_hook_honours_no_auto_resume_opt_out() {
    let settings = settings_json();
    let cmd = session_start_command(&settings);
    assert!(
        cmd.contains("CONVERGIO_NO_AUTO_RESUME"),
        "SessionStart hook should reference CONVERGIO_NO_AUTO_RESUME escape hatch: {cmd}"
    );
}

/// Smoke: run a bash harness that mirrors the hook's shape. We
/// stub the `cvg` binary with a tiny shell function so the test
/// does not need cargo, the daemon, or network. The point is to
/// prove the marker-and-fallback logic in the hook command string
/// behaves as documented.
fn run_hook_harness(no_auto_resume: bool) -> (String, String) {
    // Mirror the production hook body but with stubbed cvg calls.
    // Each stubbed step prints a unique sentinel so we can assert
    // ordering and presence/absence in the captured stdout.
    let script = r#"
        cvg() {
            case "$2" in
                register-and-poll) echo "[stub] register-and-poll fired" ;;
                resume)            echo "[stub] resume fired" ;;
                *) echo "[stub] unknown: $*" ;;
            esac
        }
        echo "=== Convergio session bootstrap ==="
        cvg session register-and-poll --agent-id "claude-code-${USER:-tester}" --kind claude --output human || true
        if [ "${CONVERGIO_NO_AUTO_RESUME:-0}" != "1" ]; then
          echo
          echo "=== Convergio session resume (live state) ==="
          cvg session resume --output plain || true
        fi
    "#;

    let mut cmd = Command::new("bash");
    cmd.arg("-c").arg(script);
    if no_auto_resume {
        cmd.env("CONVERGIO_NO_AUTO_RESUME", "1");
    } else {
        cmd.env_remove("CONVERGIO_NO_AUTO_RESUME");
    }
    let out = cmd.output().expect("spawn bash");
    assert!(out.status.success(), "harness failed: {:?}", out.status);
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn smoke_hook_prints_both_markers_and_both_stubs() {
    let (stdout, _stderr) = run_hook_harness(false);
    assert!(
        stdout.contains(BOOTSTRAP_MARKER),
        "missing bootstrap marker:\n{stdout}"
    );
    assert!(
        stdout.contains(RESUME_MARKER),
        "missing resume marker:\n{stdout}"
    );
    assert!(
        stdout.contains("[stub] register-and-poll fired"),
        "register-and-poll stub did not fire:\n{stdout}"
    );
    assert!(
        stdout.contains("[stub] resume fired"),
        "resume stub did not fire:\n{stdout}"
    );

    // Order: bootstrap marker must precede resume marker.
    let bootstrap_at = stdout.find(BOOTSTRAP_MARKER).unwrap();
    let resume_at = stdout.find(RESUME_MARKER).unwrap();
    assert!(
        bootstrap_at < resume_at,
        "bootstrap marker should come before resume marker:\n{stdout}"
    );
}

#[test]
fn smoke_hook_skips_resume_when_opt_out_is_set() {
    let (stdout, _stderr) = run_hook_harness(true);
    assert!(
        stdout.contains(BOOTSTRAP_MARKER),
        "bootstrap marker should still print when opt-out is set:\n{stdout}"
    );
    assert!(
        stdout.contains("[stub] register-and-poll fired"),
        "register-and-poll should still fire when opt-out is set:\n{stdout}"
    );
    assert!(
        !stdout.contains(RESUME_MARKER),
        "resume marker should be SUPPRESSED by CONVERGIO_NO_AUTO_RESUME=1:\n{stdout}"
    );
    assert!(
        !stdout.contains("[stub] resume fired"),
        "resume should not fire when opt-out is set:\n{stdout}"
    );
}
