//! `cvg pr link <pr-number> --plan <plan-id>` — record a PR↔plan
//! mapping in `plan_pr_links` so the daemon can trace which agent
//! opened which PR. Sub-agents call this immediately after
//! `gh pr create` so the table is populated via the "known pattern"
//! (explicit agent registration) path.
//!
//! See P2-3 / F47 in the 2026-05-04 retrospective.

use super::{Client, OutputMode};
use anyhow::{Context, Result};
use clap::Args;
use convergio_i18n::Bundle;
use serde_json::{json, Value};

/// `cvg pr link` arguments.
#[derive(Debug, Clone, Args)]
pub struct LinkArgs {
    /// GitHub PR number to register.
    #[arg(value_name = "PR_NUMBER")]
    pub pr_number: i64,
    /// Plan id (UUID or number) this PR belongs to.
    #[arg(long, value_name = "PLAN_ID")]
    pub plan: String,
    /// Task id this PR closes (optional).
    #[arg(long, value_name = "TASK_ID")]
    pub task: Option<String>,
    /// `owner/repo` slug. Defaults to the current repo via `gh repo view`.
    #[arg(long, value_name = "SLUG")]
    pub repo: Option<String>,
    /// Branch name (best-effort).
    #[arg(long, value_name = "BRANCH")]
    pub branch: Option<String>,
    /// Agent id to record. Falls back to `CONVERGIO_AGENT_ID` env var.
    #[arg(long, env = "CONVERGIO_AGENT_ID", value_name = "AGENT_ID")]
    pub agent_id: Option<String>,
}

/// Register a PR↔plan link in the daemon.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    args: LinkArgs,
) -> Result<()> {
    let repo_slug = args.repo.unwrap_or_else(detect_repo_slug_or_unknown);

    let body = json!({
        "pr_number": args.pr_number,
        "repo_slug": repo_slug,
        "branch":    args.branch,
        "task_id":   args.task,
        "agent_id":  args.agent_id,
    });

    let result: Value = client
        .post(&format!("/v1/plans/{}/pr-links", args.plan), &body)
        .await
        .with_context(|| {
            format!(
                "POST /v1/plans/{}/pr-links (PR #{})",
                args.plan, args.pr_number
            )
        })?;

    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&result)?),
        OutputMode::Plain => {
            println!(
                "pr={} plan={} repo={}",
                args.pr_number, args.plan, repo_slug
            )
        }
        OutputMode::Human => {
            let pr = args.pr_number.to_string();
            println!(
                "{}",
                bundle.t(
                    "pr-link-success",
                    &[("pr", &pr), ("plan", &args.plan), ("repo", &repo_slug)],
                )
            );
        }
    }
    Ok(())
}

/// Best-effort repo slug; falls back to `"unknown/unknown"` on failure.
pub fn detect_repo_slug_or_unknown() -> String {
    detect_repo_slug().unwrap_or_else(|| "unknown/unknown".into())
}

/// Best-effort: run `gh repo view --json nameWithOwner` to get the
/// current repo slug. Returns `None` on any failure so callers can
/// fall back gracefully.
fn detect_repo_slug() -> Option<String> {
    let out = std::process::Command::new("gh")
        .args([
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "-q",
            ".nameWithOwner",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_args_parse_minimal() {
        use clap::Parser;

        #[derive(Parser)]
        struct Cmd {
            #[command(flatten)]
            args: LinkArgs,
        }

        let cmd = Cmd::try_parse_from(["cvg", "42", "--plan", "my-plan-id"]).unwrap();
        assert_eq!(cmd.args.pr_number, 42);
        assert_eq!(cmd.args.plan, "my-plan-id");
        assert!(cmd.args.task.is_none());
        assert!(cmd.args.agent_id.is_none());
    }

    #[test]
    fn link_args_parse_full() {
        use clap::Parser;

        #[derive(Parser)]
        struct Cmd {
            #[command(flatten)]
            args: LinkArgs,
        }

        let cmd = Cmd::try_parse_from([
            "cvg",
            "7",
            "--plan",
            "plan-abc",
            "--task",
            "task-xyz",
            "--repo",
            "owner/repo",
            "--branch",
            "feat/my-branch",
            "--agent-id",
            "agent-1",
        ])
        .unwrap();
        assert_eq!(cmd.args.pr_number, 7);
        assert_eq!(cmd.args.plan, "plan-abc");
        assert_eq!(cmd.args.task.as_deref(), Some("task-xyz"));
        assert_eq!(cmd.args.repo.as_deref(), Some("owner/repo"));
        assert_eq!(cmd.args.branch.as_deref(), Some("feat/my-branch"));
        assert_eq!(cmd.args.agent_id.as_deref(), Some("agent-1"));
    }
}
