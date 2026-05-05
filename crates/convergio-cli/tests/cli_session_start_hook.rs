//! Tests for the project-level Claude Code `SessionStart` hook
//! in `.claude/settings.json` (P2-6).
//!
//! The hook must run `cvg session register-and-poll` (existing,
//! #171) AND `cvg session resume --output plain`, print both
//! blocks behind clear markers, and honour `CONVERGIO_NO_AUTO_RESUME=1`
//! as the opt-out.

use std::path::PathBuf;
use std::process::Command;

const BOOTSTRAP_MARKER: &str = "=== Convergio session bootstrap ===";
const RESUME_MARKER: &str = "=== Convergio session resume (live state) ===";

fn workspace_root() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(|p| p.parent())
        .expect("workspace root above crates/convergio-cli")
        .to_path_buf()
}

fn session_start_command() -> String {
    let path = workspace_root().join(".claude").join("settings.json");
    let raw =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let v: serde_json::Value = serde_json::from_str(&raw).expect("parse settings.json");
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
        "no SessionStart command in settings.json"
    );
    commands.join("\n")
}

#[test]
fn session_start_hook_settings_are_well_formed() {
    let cmd = session_start_command();
    for needle in [
        "session register-and-poll",
        "session resume",
        "--output plain",
        BOOTSTRAP_MARKER,
        RESUME_MARKER,
        "CONVERGIO_NO_AUTO_RESUME",
    ] {
        assert!(
            cmd.contains(needle),
            "SessionStart hook missing {needle:?}: {cmd}"
        );
    }
}

/// Run a bash harness mirroring the hook's shape, with `cvg`
/// stubbed by a shell function so the test needs no cargo,
/// daemon, or network.
fn run_hook_harness(no_auto_resume: bool) -> String {
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
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn smoke_hook_prints_both_markers_in_order() {
    let stdout = run_hook_harness(false);
    for needle in [
        BOOTSTRAP_MARKER,
        RESUME_MARKER,
        "[stub] register-and-poll fired",
        "[stub] resume fired",
    ] {
        assert!(stdout.contains(needle), "missing {needle:?}:\n{stdout}");
    }
    let bootstrap_at = stdout.find(BOOTSTRAP_MARKER).unwrap();
    let resume_at = stdout.find(RESUME_MARKER).unwrap();
    assert!(
        bootstrap_at < resume_at,
        "bootstrap must precede resume:\n{stdout}"
    );
}

#[test]
fn smoke_hook_opt_out_skips_resume_only() {
    let stdout = run_hook_harness(true);
    for present in [BOOTSTRAP_MARKER, "[stub] register-and-poll fired"] {
        assert!(stdout.contains(present), "missing {present:?}:\n{stdout}");
    }
    for absent in [RESUME_MARKER, "[stub] resume fired"] {
        assert!(
            !stdout.contains(absent),
            "should be suppressed: {absent:?}\n{stdout}"
        );
    }
}
