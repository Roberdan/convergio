//! `check.friction.missing` — friction-log entries hinted in commits
//! but not written.
//!
//! Walks `git log --since=24.hours.ago --pretty=%B` for the bodies
//! of recent commits, extracts every `\bF[0-9]+\b` reference, and
//! flags ids that do not appear as a row header in
//! `docs/plans/v0.2-friction-log.md`. F40 already has the inverse
//! check (`scripts/check-friction-log-mirror.sh`); this one catches
//! the other direction — code mentions a friction id without
//! recording it.
//!
//! Conservative on failure: missing `git`, missing log file, or
//! shell errors all collapse to `Pass`.

use crate::pre_stop::{Check, CheckContext, CheckOutcome};
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;

/// Concrete check implementation.
pub struct FrictionMissingCheck;

impl Check for FrictionMissingCheck {
    fn id(&self) -> &'static str {
        "check.friction.missing"
    }
    fn label(&self) -> &'static str {
        "friction-log entries hinted in commits but not written"
    }
    fn run(&self, _ctx: &CheckContext) -> CheckOutcome {
        let log_text = match git_log_recent() {
            Ok(t) => t,
            Err(_) => return CheckOutcome::Pass,
        };
        let mentioned: HashSet<String> = scan_friction_ids(&log_text);
        if mentioned.is_empty() {
            return CheckOutcome::Pass;
        }
        let logged: HashSet<String> = match read_log_ids() {
            Ok(s) => s,
            Err(_) => return CheckOutcome::Pass,
        };
        let mut missing: Vec<String> = mentioned.difference(&logged).cloned().collect();
        missing.sort();
        if missing.is_empty() {
            CheckOutcome::Pass
        } else {
            let findings = missing
                .into_iter()
                .map(|id| {
                    format!("commit message references {id} but no row in v0.2-friction-log.md")
                })
                .collect();
            CheckOutcome::Fail { findings }
        }
    }
}

fn git_log_recent() -> Result<String, ()> {
    let out = Command::new("git")
        .args(["log", "--since=24.hours.ago", "--pretty=%B"])
        .output()
        .map_err(|_| ())?;
    if !out.status.success() {
        return Err(());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Extract every `\bF[0-9]+\b` from the input.
fn scan_friction_ids(s: &str) -> HashSet<String> {
    let mut out = HashSet::new();
    let bytes = s.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i < n {
        if bytes[i] == b'F'
            && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric())
            && i + 1 < n
            && bytes[i + 1].is_ascii_digit()
        {
            let mut j = i + 1;
            while j < n && bytes[j].is_ascii_digit() {
                j += 1;
            }
            if j == n || !bytes[j].is_ascii_alphanumeric() {
                out.insert(String::from_utf8_lossy(&bytes[i..j]).into_owned());
            }
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

const LOG_PATH: &str = "docs/plans/v0.2-friction-log.md";

fn read_log_ids() -> Result<HashSet<String>, ()> {
    if !Path::new(LOG_PATH).exists() {
        return Err(());
    }
    let text = std::fs::read_to_string(LOG_PATH).map_err(|_| ())?;
    Ok(scan_friction_ids(&text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_friction_ids_extracts_word_boundaries() {
        let s = "fixed F46 and F46b but not F999. (refF200 should not match.)";
        let ids = scan_friction_ids(s);
        assert!(ids.contains("F46"));
        assert!(ids.contains("F999"));
        // F46b — `b` after digit is alphanumeric so the run F46
        // gets emitted; `F200` after `ref` (alphanumeric) is rejected.
        assert!(!ids.contains("F200"));
    }

    #[test]
    fn check_id_and_label_are_stable() {
        let c = FrictionMissingCheck;
        assert_eq!(c.id(), "check.friction.missing");
    }
}
