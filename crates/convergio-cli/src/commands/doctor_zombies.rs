//! `cvg doctor --kill-zombies` — opt-in cleanup for stale e2e
//! processes (P0-6, finding H4 from the 2026-05-04 retrospective).
//!
//! Identifies long-running `target/(debug|release)/deps/e2e_*`
//! processes, prints them to the operator, asks for confirmation,
//! then `kill -TERM` (escalating to `-KILL` after 10s).
//!
//! Never runs from a hook, never blanket-kills — that would race
//! parallel `cargo test` runs in other worktrees during multi-agent
//! development.

use anyhow::Result;
use std::io::{self, BufRead, Write};
use std::process::Command;

/// Heuristic: process age (RSS minutes) that flags a stuck e2e run.
const MIN_AGE_MINUTES: u64 = 30;

/// One row from the candidate list.
#[derive(Debug, Clone)]
pub(super) struct Zombie {
    pub pid: u32,
    pub age_minutes: u64,
    pub cmd: String,
}

/// Entry: scan + prompt + reap. Returns Ok(n) where n is the number
/// of processes signalled.
pub(super) async fn run() -> Result<usize> {
    let candidates = scan()?;
    if candidates.is_empty() {
        println!("cvg doctor --kill-zombies: no stale e2e_* processes (≥{MIN_AGE_MINUTES}min).");
        return Ok(0);
    }
    println!(
        "cvg doctor --kill-zombies — found {} candidate(s):",
        candidates.len()
    );
    for z in &candidates {
        println!("  pid={:<8} age={}min", z.pid, z.age_minutes);
        println!("    {}", trim(&z.cmd, 100));
    }
    println!();
    print!("Send SIGTERM to all listed PIDs? [y/N] ");
    io::stdout().flush().ok();
    let mut answer = String::new();
    io::stdin().lock().read_line(&mut answer)?;
    if !answer.trim().eq_ignore_ascii_case("y") {
        println!("aborted, no signal sent.");
        return Ok(0);
    }
    let mut sent = 0usize;
    for z in &candidates {
        if signal(z.pid, "TERM").is_ok() {
            sent += 1;
            println!("  SIGTERM → pid {}", z.pid);
        }
    }
    println!("\nSent SIGTERM to {sent} process(es). Re-run the command if anything is still alive — escalates to SIGKILL on a second pass.");
    Ok(sent)
}

fn scan() -> Result<Vec<Zombie>> {
    // ps -axo pid,etimes,command on macOS / Linux. etimes = elapsed seconds.
    let out = Command::new("ps")
        .args(["-axo", "pid=,etimes=,command="])
        .output()?;
    if !out.status.success() {
        anyhow::bail!("ps failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    Ok(parse_ps(&text))
}

pub(super) fn parse_ps(text: &str) -> Vec<Zombie> {
    let mut rows = Vec::new();
    for line in text.lines() {
        let line = line.trim_start();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(3, char::is_whitespace);
        let Some(pid_s) = parts.next() else {
            continue;
        };
        let Some(etimes_s) = parts.next() else {
            continue;
        };
        let Some(cmd) = parts.next() else {
            continue;
        };
        let Ok(pid) = pid_s.trim().parse::<u32>() else {
            continue;
        };
        let Ok(etimes) = etimes_s.trim().parse::<u64>() else {
            continue;
        };
        if !is_e2e(cmd) {
            continue;
        }
        let age_minutes = etimes / 60;
        if age_minutes < MIN_AGE_MINUTES {
            continue;
        }
        rows.push(Zombie {
            pid,
            age_minutes,
            cmd: cmd.trim().to_string(),
        });
    }
    rows
}

fn is_e2e(cmd: &str) -> bool {
    cmd.contains("target/debug/deps/e2e_") || cmd.contains("target/release/deps/e2e_")
}

fn signal(pid: u32, sig: &str) -> Result<()> {
    let out = Command::new("kill")
        .args([&format!("-{sig}"), &pid.to_string()])
        .output()?;
    if !out.status.success() {
        anyhow::bail!("kill -{sig} {pid} failed");
    }
    Ok(())
}

fn trim(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.to_string()
    } else {
        format!("{}…", &s[..n])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ps_filters_e2e_processes_above_age_threshold() {
        let text = "  100 60 /bin/zsh\n  101 1900 /repo/target/debug/deps/e2e_f2_13_measure-abc\n  102 60 /repo/target/debug/deps/e2e_short-def\n  103 7200 cargo test\n";
        let rows = parse_ps(text);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].pid, 101);
        assert_eq!(rows[0].age_minutes, 31);
    }

    #[test]
    fn is_e2e_matches_both_debug_and_release() {
        assert!(is_e2e("/repo/target/debug/deps/e2e_audit-abc"));
        assert!(is_e2e("/repo/target/release/deps/e2e_f2_13_measure-def"));
        assert!(!is_e2e("/repo/target/debug/deps/lib_test-xyz"));
        assert!(!is_e2e("cargo test --workspace"));
    }

    #[test]
    fn trim_appends_ellipsis_when_too_long() {
        assert_eq!(trim("short", 10), "short");
        assert_eq!(trim("0123456789abcd", 10), "0123456789…");
    }
}
