//! `PermissionProfile` — least-privilege envelopes for spawned
//! agents.
//!
//! ADR-0033 follow-up: vendor CLIs in non-interactive mode used to
//! require `--dangerously-skip-permissions` (Claude) or
//! `--allow-all-tools` (Copilot) to even start. That is incompatible
//! with the project's first sacred principle — Convergio is the
//! *leash*, not a power-of-attorney. Both CLIs ship a granular
//! permission API; we use it.
//!
//! Profiles describe an *intent* (`Standard`, `ReadOnly`, …) and
//! each runner translates the intent into vendor-specific flags
//! (`--allowed-tools`, `--allow-tool`, `--deny-tool`, etc.).

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Logical permission profile.
///
/// `Standard` is the default for code-implementing tasks: it lets
/// the agent build, test, edit files in the worktree, talk to the
/// daemon (`cvg`), and open PRs via `gh` — but not push to `main`,
/// not `rm -rf`, not `sudo`. `ReadOnly` is for tasks that only
/// query the codebase. `Sandbox` is the explicit nuke-everything
/// opt-in for an isolated VM where the audit chain plus the
/// worktree boundary are the only safety net.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PermissionProfile {
    /// Build / edit / open PR. Default.
    #[default]
    Standard,
    /// Read-only inspection — Read / Glob / Grep, no Bash, no Edit.
    ReadOnly,
    /// Bypass everything. For a sealed sandbox only — never for
    /// the operator's main checkout.
    Sandbox,
    /// Wide-open shell allow-list (`shell(*)`) — kept under the
    /// daemon's deny-list (rm, sudo, push to main, force-push,
    /// reset --hard). Use when the operator trusts the agent and
    /// `Standard`'s prefix-only matchers are too tight (e.g. the
    /// vendor CLI wraps `git --no-pager …` so `shell(git:status*)`
    /// doesn't match).
    Unrestricted,
}

impl PermissionProfile {
    /// Human-friendly tag (also the CLI flag value).
    pub fn tag(self) -> &'static str {
        match self {
            PermissionProfile::Standard => "standard",
            PermissionProfile::ReadOnly => "read_only",
            PermissionProfile::Sandbox => "sandbox",
            PermissionProfile::Unrestricted => "unrestricted",
        }
    }

    /// Claude's `--allowed-tools` value for this profile.
    /// Returns `None` for `Sandbox` (caller emits
    /// `--dangerously-skip-permissions` instead).
    pub fn claude_allowed_tools(self) -> Option<&'static str> {
        match self {
            PermissionProfile::Standard => Some(
                "Read Glob Grep Edit Write TodoWrite \
                 Bash(cargo *) \
                 Bash(git status) Bash(git status:*) \
                 Bash(git diff) Bash(git diff:*) \
                 Bash(git log) Bash(git log:*) \
                 Bash(git branch) Bash(git branch:*) \
                 Bash(git checkout) Bash(git checkout:*) \
                 Bash(git add) Bash(git add:*) \
                 Bash(git commit) Bash(git commit:*) \
                 Bash(git push origin :*) \
                 Bash(git fetch) Bash(git fetch:*) \
                 Bash(git rebase) Bash(git rebase:*) \
                 Bash(git worktree:*) \
                 Bash(cvg *) \
                 Bash(gh pr:*) Bash(gh api repos/*) Bash(gh run:*) \
                 Bash(jq:*) Bash(rg:*) Bash(grep:*) Bash(awk:*) \
                 Bash(ls) Bash(ls:*) Bash(cat) Bash(cat:*) \
                 Bash(pwd) Bash(which:*) \
                 Bash(mkdir) Bash(mkdir:*) Bash(touch:*) \
                 Bash(bash scripts/*) Bash(./scripts/*)",
            ),
            PermissionProfile::ReadOnly => Some("Read Glob Grep TodoWrite"),
            PermissionProfile::Sandbox => None,
            // Same shape as Sandbox — Claude has no granular wildcard
            // beyond `--dangerously-skip-permissions`. The daemon's
            // deny-list still applies via the Bash tool's command
            // filter on Copilot side; on Claude it is the operator's
            // worktree boundary that contains blast radius.
            PermissionProfile::Unrestricted => None,
        }
    }

    /// Claude's `--permission-mode`.
    pub fn claude_permission_mode(self) -> &'static str {
        match self {
            // `acceptEdits` auto-accepts file edits within the
            // worktree; everything else falls back to the
            // allowed-tools whitelist.
            PermissionProfile::Standard => "acceptEdits",
            PermissionProfile::ReadOnly => "default",
            PermissionProfile::Sandbox => "bypassPermissions",
            // Same as Sandbox at the Claude flag level — see
            // [`Self::claude_allowed_tools`] for the rationale.
            PermissionProfile::Unrestricted => "bypassPermissions",
        }
    }

    /// Copilot's `--allow-tool` patterns.
    pub fn copilot_allow_tools(self) -> Vec<&'static str> {
        match self {
            PermissionProfile::Standard => vec![
                "write",
                "shell(cargo:*)",
                "shell(git:status*)",
                "shell(git:diff*)",
                "shell(git:log*)",
                "shell(git:branch*)",
                "shell(git:checkout*)",
                "shell(git:add*)",
                "shell(git:commit*)",
                "shell(git:fetch*)",
                "shell(git:rebase*)",
                "shell(git:worktree*)",
                "shell(cvg:*)",
                "shell(gh:pr*)",
                "shell(gh:api*)",
                "shell(gh:run*)",
                "shell(jq:*)",
                "shell(rg:*)",
                "shell(ls*)",
                "shell(cat*)",
                "shell(pwd)",
                "shell(which*)",
                "shell(mkdir*)",
                "shell(touch*)",
            ],
            PermissionProfile::ReadOnly => vec!["shell(rg:*)", "shell(ls*)", "shell(cat*)"],
            PermissionProfile::Sandbox => vec![],
            // Wildcard `shell(*)` covers everything the deny-list
            // doesn't already block. Use when the prefix-matchers
            // in `Standard` are too narrow for the vendor CLI's
            // command shape (e.g. `git --no-pager status` is *not*
            // matched by `shell(git:status*)`).
            PermissionProfile::Unrestricted => vec!["write", "shell(*)"],
        }
    }

    /// Copilot's `--deny-tool` patterns. Always applied, even on
    /// `Sandbox` — the audit chain forbids these forever.
    pub fn copilot_deny_tools(self) -> Vec<&'static str> {
        vec![
            "shell(rm:*)",
            "shell(sudo*)",
            "shell(git:push origin main*)",
            "shell(git:push --force*)",
            "shell(git:reset --hard*)",
            "shell(curl:* -d *)",
            "shell(chmod:777*)",
        ]
    }
}

impl FromStr for PermissionProfile {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "standard" => Ok(PermissionProfile::Standard),
            "read_only" | "read-only" | "readonly" => Ok(PermissionProfile::ReadOnly),
            "sandbox" => Ok(PermissionProfile::Sandbox),
            "unrestricted" => Ok(PermissionProfile::Unrestricted),
            other => Err(format!("unknown profile `{other}`")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_allows_cargo_and_cvg() {
        let allowed = PermissionProfile::Standard.claude_allowed_tools().unwrap();
        assert!(allowed.contains("Bash(cargo *)"));
        assert!(allowed.contains("Bash(cvg *)"));
        assert!(allowed.contains("Read"));
        assert!(allowed.contains("Edit"));
    }

    #[test]
    fn standard_does_not_open_arbitrary_shell() {
        // The literal `Bash(*)` wildcard would be a regression.
        let allowed = PermissionProfile::Standard.claude_allowed_tools().unwrap();
        assert!(!allowed.contains("Bash(*)"));
    }

    #[test]
    fn read_only_blocks_edits() {
        let allowed = PermissionProfile::ReadOnly.claude_allowed_tools().unwrap();
        // Token-level contains: split on whitespace + check.
        let tokens: Vec<&str> = allowed.split_whitespace().collect();
        assert!(tokens.contains(&"Read"));
        assert!(!tokens.contains(&"Edit"));
        assert!(!tokens.contains(&"Write"));
        assert!(!tokens.iter().any(|t| t.starts_with("Bash")));
        // TodoWrite is the agent's internal scratchpad, not a file
        // write — keep it allowed even in ReadOnly.
        assert!(tokens.contains(&"TodoWrite"));
    }

    #[test]
    fn sandbox_returns_no_whitelist() {
        assert_eq!(PermissionProfile::Sandbox.claude_allowed_tools(), None);
        assert_eq!(
            PermissionProfile::Sandbox.claude_permission_mode(),
            "bypassPermissions"
        );
    }

    #[test]
    fn copilot_deny_list_includes_destructive_commands() {
        let deny = PermissionProfile::Standard.copilot_deny_tools();
        assert!(deny.iter().any(|t| t.contains("rm:")));
        assert!(deny.iter().any(|t| t.contains("sudo")));
        assert!(deny.iter().any(|t| t.contains("push origin main")));
    }

    #[test]
    fn from_str_round_trips() {
        for p in [
            PermissionProfile::Standard,
            PermissionProfile::ReadOnly,
            PermissionProfile::Sandbox,
            PermissionProfile::Unrestricted,
        ] {
            assert_eq!(PermissionProfile::from_str(p.tag()).unwrap(), p);
        }
        assert!(PermissionProfile::from_str("nonsense").is_err());
    }

    #[test]
    fn unrestricted_uses_wildcard_shell_and_keeps_deny_list() {
        let allow = PermissionProfile::Unrestricted.copilot_allow_tools();
        assert!(allow.contains(&"shell(*)"));
        assert!(allow.contains(&"write"));
        // The deny list is profile-independent — destructive commands
        // stay blocked even under `Unrestricted`.
        let deny = PermissionProfile::Unrestricted.copilot_deny_tools();
        assert!(deny.iter().any(|t| t.contains("rm:")));
        assert!(deny.iter().any(|t| t.contains("push origin main")));
    }

    #[test]
    fn unrestricted_drops_claude_allow_list_for_bypass_mode() {
        // Claude exposes no shell wildcard short of bypassPermissions.
        assert_eq!(PermissionProfile::Unrestricted.claude_allowed_tools(), None);
        assert_eq!(
            PermissionProfile::Unrestricted.claude_permission_mode(),
            "bypassPermissions"
        );
    }
}
