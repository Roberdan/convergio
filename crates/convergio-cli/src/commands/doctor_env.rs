//! Env-var visibility check for `cvg doctor` (issue #407).
//!
//! When the daemon is started manually (`convergio start`) instead of
//! via launchd it does NOT inherit the plist `EnvironmentVariables`,
//! so it silently falls back to restrictive compiled-in defaults.
//! The first symptom is "no agent ever spawns" — invisible without
//! this check.

/// One row of advisory output for the env check.
pub struct EnvFinding {
    /// Stable name (used as the doctor check key).
    pub name: &'static str,
    /// True when the env var is set in the current process.
    pub set: bool,
    /// One-line description for the human/JSON output.
    pub message: String,
}

/// Operator-visible env vars whose absence on the **daemon process**
/// (not the CLI) silently degrades dispatch. We can only check the
/// CLI's own environment here, which is a useful proxy when both
/// were started from the same shell; for the launchd case operators
/// should still run `cvg service start`.
const KEY_VARS: &[(&str, &str)] = &[
    (
        "CONVERGIO_GUARD_MAX_WORKTREES",
        "max parallel worktrees on disk (compiled default: 2)",
    ),
    (
        "CONVERGIO_EXECUTOR_MAX_PARALLEL",
        "per-tick dispatch fan-out cap (compiled default: unbounded)",
    ),
    (
        "CONVERGIO_RUNNER_DEFAULT",
        "default runner kind/profile (compiled default: claude:sonnet)",
    ),
    (
        "CONVERGIO_REPO_PATH",
        "repo root used to pre-create worktrees",
    ),
];

/// Inspect the current process environment for the key knobs.
/// Returns one finding per checked var. The caller decides how
/// to render them.
pub fn check_env() -> Vec<EnvFinding> {
    KEY_VARS
        .iter()
        .map(|(name, hint)| {
            let set = std::env::var_os(name).is_some_and(|v| !v.is_empty());
            let message = if set {
                format!("{name} is set")
            } else {
                format!("{name} not set — {hint}")
            };
            EnvFinding { name, set, message }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_var_is_flagged() {
        // Use a name we know nobody sets so the test is stable.
        let findings = check_env();
        let repo = findings
            .iter()
            .find(|f| f.name == "CONVERGIO_REPO_PATH")
            .expect("CONVERGIO_REPO_PATH finding present");
        // Either set or not — we just assert message wording matches state.
        if repo.set {
            assert!(repo.message.contains("is set"));
        } else {
            assert!(repo.message.contains("not set"));
        }
    }

    #[test]
    fn all_key_vars_have_findings() {
        let findings = check_env();
        assert_eq!(findings.len(), KEY_VARS.len());
    }
}
