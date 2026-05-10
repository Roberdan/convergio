//! `/v1/pr-links` — read PR ownership links.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use convergio_durability::PlanPrLink;
use serde::Deserialize;

/// Mount PR link routes.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/pr-links", get(list))
}

#[derive(Deserialize)]
struct ListQuery {
    #[serde(default)]
    repo_slug: Option<String>,
    #[serde(default)]
    pr_number: Option<i64>,
    #[serde(default)]
    pr_url: Option<String>,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_limit() -> i64 {
    50
}

async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<PlanPrLink>>, ApiError> {
    let (repo_slug, pr_number) = if let Some(url) = q.pr_url.as_deref() {
        parse_github_pr_url(url).ok_or_else(|| ApiError::BadRequest {
            code: "invalid_pr_url",
            message: format!("invalid GitHub PR URL: {url}"),
        })?
    } else {
        let repo_slug = q.repo_slug.clone().ok_or_else(|| ApiError::BadRequest {
            code: "missing_repo_slug",
            message: "repo_slug is required (or pass pr_url)".into(),
        })?;
        let pr_number = q.pr_number.ok_or_else(|| ApiError::BadRequest {
            code: "missing_pr_number",
            message: "pr_number is required (or pass pr_url)".into(),
        })?;
        (repo_slug, pr_number)
    };

    let links = state
        .durability
        .plan_pr_links()
        .list_by_pr(&repo_slug, pr_number, q.limit)
        .await?;

    Ok(Json(links))
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
