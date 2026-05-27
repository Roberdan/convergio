//! W11 local-stub axe-core wrapper for Convergio A11yGate phase 2.
//!
//! This crate exists to unblock W11 of
//! `docs/plans/v1.0-production-ready.md` **without** requiring the
//! remote capability registry (W9 follow-up). It provides a tiny
//! opt-in wrapper around an external `axe` binary. When the binary is
//! not configured or not on PATH, every call returns
//! [`AxeStatus::NotConfigured`] — the gate caller is then expected to
//! skip phase-2 checks and rely on the phase-1 built-in subset
//! (`convergio_durability::gates::a11y_gate`).
//!
//! Rationale and contract: see [ADR-0064](../../docs/adr/0064-a11y-axe-local-stub.md).
//!
//! Opt-in is **explicit** — there is no implicit shell-out. The
//! caller must set `CONVERGIO_A11Y_AXE_BIN` to the absolute path of
//! an `axe` (or compatible) binary before invoking [`run_html`].
//!
//! # Example
//!
//! ```no_run
//! use convergio_a11y_axe::{run_html, AxeStatus};
//!
//! match run_html("<html><body><h1>hi</h1></body></html>") {
//!     AxeStatus::NotConfigured => {
//!         // phase-1 only — fall through
//!     }
//!     AxeStatus::Ok(report) if report.violations.is_empty() => {
//!         // pass
//!     }
//!     AxeStatus::Ok(report) => {
//!         eprintln!("a11y violations: {}", report.violations.len());
//!     }
//!     AxeStatus::Error(err) => {
//!         eprintln!("axe runner failed: {err}");
//!     }
//! }
//! ```

#![deny(missing_docs)]

use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Environment variable that opts the wrapper in.
///
/// Set to the absolute path of an `axe` binary (or any binary that
/// accepts HTML on stdin and prints a JSON report compatible with
/// [`AxeReport`] on stdout).
pub const AXE_BIN_ENV: &str = "CONVERGIO_A11Y_AXE_BIN";

/// Single axe-core violation, in the deterministic shape we expect on
/// the binary's stdout.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct AxeViolation {
    pub id: String,
    pub impact: String,
    pub help: String,
}

/// Report returned by the external axe binary.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[allow(missing_docs)]
pub struct AxeReport {
    pub violations: Vec<AxeViolation>,
}

/// Outcome of an axe run.
#[derive(Debug)]
pub enum AxeStatus {
    /// `CONVERGIO_A11Y_AXE_BIN` unset or pointing nowhere; caller
    /// should fall back to phase-1 built-in checks.
    NotConfigured,
    /// Binary ran and produced a report (possibly with violations).
    Ok(AxeReport),
    /// Binary configured but the invocation failed (non-zero exit,
    /// I/O error, malformed JSON, etc.).
    Error(String),
}

/// Returns `Some(path)` when the env var is set and the path exists.
pub fn configured_binary() -> Option<PathBuf> {
    let raw = env::var(AXE_BIN_ENV).ok()?;
    let path = PathBuf::from(raw);
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

/// Run the configured axe binary against the given HTML snippet.
///
/// Returns [`AxeStatus::NotConfigured`] when [`configured_binary`]
/// returns `None`, so the caller can cheaply check without branching
/// on env state twice.
pub fn run_html(html: &str) -> AxeStatus {
    let Some(bin) = configured_binary() else {
        return AxeStatus::NotConfigured;
    };
    let mut child = match Command::new(&bin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return AxeStatus::Error(format!("spawn {bin:?}: {e}")),
    };
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        if let Err(e) = stdin.write_all(html.as_bytes()) {
            return AxeStatus::Error(format!("write stdin: {e}"));
        }
    }
    let out = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return AxeStatus::Error(format!("wait: {e}")),
    };
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return AxeStatus::Error(format!(
            "axe exit {:?}: {}",
            out.status.code(),
            stderr.trim()
        ));
    }
    match serde_json::from_slice::<AxeReport>(&out.stdout) {
        Ok(r) => AxeStatus::Ok(r),
        Err(e) => AxeStatus::Error(format!("parse report: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_configured_when_env_unset() {
        // Save & clear the env, restore after.
        let prev = env::var(AXE_BIN_ENV).ok();
        // SAFETY: single-threaded test; env::remove_var is unsafe in
        // edition 2024 but acceptable here.
        unsafe { env::remove_var(AXE_BIN_ENV) };
        let status = run_html("<html></html>");
        if let Some(v) = prev {
            unsafe { env::set_var(AXE_BIN_ENV, v) };
        }
        assert!(matches!(status, AxeStatus::NotConfigured));
    }

    #[test]
    fn configured_binary_returns_none_for_missing_path() {
        let prev = env::var(AXE_BIN_ENV).ok();
        unsafe { env::set_var(AXE_BIN_ENV, "/definitely/not/a/real/axe") };
        let got = configured_binary();
        match prev {
            Some(v) => unsafe { env::set_var(AXE_BIN_ENV, v) },
            None => unsafe { env::remove_var(AXE_BIN_ENV) },
        }
        assert!(got.is_none());
    }

    #[test]
    fn report_round_trips_via_serde() {
        let r = AxeReport {
            violations: vec![AxeViolation {
                id: "color-contrast".into(),
                impact: "serious".into(),
                help: "Insufficient contrast".into(),
            }],
        };
        let s = serde_json::to_string(&r).unwrap();
        let back: AxeReport = serde_json::from_str(&s).unwrap();
        assert_eq!(r, back);
    }
}
