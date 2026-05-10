//! `cvg pr who <url|number>` — show which agent opened a PR.

use super::pr_link::detect_repo_slug_or_unknown;
use super::{Client, OutputMode};
use anyhow::{Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};

/// `cvg pr who` arguments.
#[derive(Debug, Clone, Args)]
pub struct WhoArgs {
    /// PR URL (`https://github.com/owner/repo/pull/123`) or PR number.
    #[arg(value_name = "PR")]
    pub pr: String,

    /// `owner/repo` slug. Optional when PR is a URL; otherwise defaults
    /// to the current repo via `gh repo view`.
    #[arg(long, value_name = "SLUG")]
    pub repo: Option<String>,

    /// Max rows to show.
    #[arg(long, default_value_t = 10)]
    pub limit: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PrLinkRow {
    pub plan_id: String,
    pub task_id: Option<String>,
    pub agent_id: Option<String>,
    pub repo_slug: String,
    pub pr_number: i64,
    pub branch: Option<String>,
    pub created_at: String,
}

/// Run `cvg pr who`.
pub async fn run(client: &Client, output: OutputMode, args: WhoArgs) -> Result<()> {
    let (repo_slug, pr_number) = parse_pr_arg(&args.pr, args.repo.as_deref())?;

    let path = format!(
        "/v1/pr-links?repo_slug={repo_slug}&pr_number={pr_number}&limit={}",
        args.limit
    );
    let links: Vec<PrLinkRow> = client.get(&path).await.with_context(|| {
        format!("GET /v1/pr-links (repo_slug={repo_slug} pr_number={pr_number})")
    })?;

    match output {
        OutputMode::Json => {
            println!("{}", serde_json::to_string_pretty(&links)?);
        }
        OutputMode::Plain => {
            let who = links
                .first()
                .and_then(|l| l.agent_id.as_deref())
                .unwrap_or("unknown");
            println!("{who}");
        }
        OutputMode::Human => {
            if links.is_empty() {
                println!("No PR ownership recorded for {repo_slug}#{pr_number}");
                return Ok(());
            }
            let top = &links[0];
            println!(
                "{}#{pr_number} → agent={} plan={} task={}{}",
                repo_slug,
                top.agent_id.as_deref().unwrap_or("unknown"),
                top.plan_id,
                top.task_id.as_deref().unwrap_or("-"),
                top.branch
                    .as_deref()
                    .map(|b| format!(" branch={b}"))
                    .unwrap_or_default(),
            );
            if links.len() > 1 {
                println!("(showing {} latest links)", links.len());
            }
        }
    }

    Ok(())
}

fn parse_pr_arg(pr: &str, repo: Option<&str>) -> Result<(String, i64)> {
    if let Some((slug, num)) = parse_github_pr_url(pr) {
        return Ok((slug, num));
    }

    let pr_number: i64 = pr
        .trim()
        .parse()
        .with_context(|| format!("PR must be a URL or integer: {pr}"))?;

    let repo_slug = repo
        .map(str::to_string)
        .unwrap_or_else(detect_repo_slug_or_unknown);

    Ok((repo_slug, pr_number))
}

fn parse_github_pr_url(url: &str) -> Option<(String, i64)> {
    let url = url.trim();
    let url = url
        .strip_prefix("https://github.com/")
        .or_else(|| url.strip_prefix("http://github.com/"))?;

    let mut parts = url.split('/');
    let owner = parts.next()?.trim();
    let repo = parts.next()?.trim();
    let kind = parts.next()?;
    if kind != "pull" {
        return None;
    }
    let pr_raw = parts.next()?;
    let pr_number_str = pr_raw.split(['?', '#']).next().unwrap_or("");
    let pr_number = pr_number_str.parse::<i64>().ok()?;

    if owner.is_empty() || repo.is_empty() {
        return None;
    }

    Some((format!("{owner}/{repo}"), pr_number))
}
