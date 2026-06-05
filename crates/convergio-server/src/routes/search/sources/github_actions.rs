use super::super::util::{href, matches_ci, score_fields, upsert};
use super::super::SearchResult;
use serde::Deserialize;
use std::collections::HashMap;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const RUN_LIMIT: &str = "30";
const GH_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug, Deserialize)]
struct GhRun {
    #[serde(rename = "databaseId")]
    id: i64,
    #[serde(rename = "workflowName")]
    workflow_name: Option<String>,
    #[serde(rename = "displayTitle")]
    display_title: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
    #[serde(rename = "headBranch")]
    head_branch: Option<String>,
    event: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhRunView {
    #[serde(default)]
    jobs: Vec<GhJob>,
    #[serde(rename = "workflowName")]
    workflow_name: Option<String>,
    #[serde(rename = "displayTitle")]
    display_title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GhJob {
    #[serde(rename = "databaseId")]
    id: i64,
    name: Option<String>,
    status: Option<String>,
    conclusion: Option<String>,
}

pub(super) async fn collect_github_actions(
    query: &str,
    merged: &mut HashMap<(String, String), SearchResult>,
) {
    let mut matched_run_ids: Vec<i64> = Vec::new();

    for status in ["in_progress", "queued"] {
        let runs = match list_runs(status).await {
            Ok(r) => r,
            Err(e) => {
                tracing::debug!(error = %e, status, "gh run list failed; skipping GitHub Actions source");
                return;
            }
        };

        for run in runs {
            let id_str = run.id.to_string();
            let id = id_str.as_str();
            let workflow_name = run.workflow_name.as_deref().unwrap_or("");
            let display_title = run.display_title.as_deref().unwrap_or("");
            let head_branch = run.head_branch.as_deref().unwrap_or("");
            let event = run.event.as_deref().unwrap_or("");
            if !matches_ci(
                query,
                [id, workflow_name, display_title, head_branch, event],
            ) {
                continue;
            }
            let score = score_fields(query, [id, display_title, workflow_name]);

            let title = run
                .display_title
                .clone()
                .or(run.workflow_name.clone())
                .unwrap_or_else(|| format!("run {id_str}"));
            let workflow = run.workflow_name.unwrap_or_else(|| "workflow".into());
            let status = run.status.unwrap_or_else(|| status.into());
            let conclusion = run.conclusion.unwrap_or_else(|| "".into());
            let branch = run.head_branch.unwrap_or_default();

            let subtitle = Some(format!(
                "{workflow} · {status} {conclusion} · {branch}",
                conclusion = conclusion
            ));

            upsert(
                merged,
                SearchResult {
                    kind: "gh_run".into(),
                    id: id_str,
                    title,
                    subtitle,
                    href: href("gh_run", &run.id.to_string()),
                    score: 46.0 + score,
                    match_sources: vec!["operational".into()],
                },
            );

            matched_run_ids.push(run.id);
        }
    }

    // Jobs are more expensive (one `gh run view` per run). Fetch a small
    // number for the runs that already matched the query.
    matched_run_ids.sort();
    matched_run_ids.dedup();
    for run_id in matched_run_ids.into_iter().take(3) {
        let view = match run_view(run_id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!(error = %e, run_id, "gh run view failed; skipping job hits");
                continue;
            }
        };

        let run_title = view
            .display_title
            .clone()
            .or(view.workflow_name.clone())
            .unwrap_or_else(|| format!("run {run_id}"));

        for job in view.jobs {
            let Some(status) = job.status.as_deref() else {
                continue;
            };
            // Only show open jobs for the unified operational surface.
            if status.eq_ignore_ascii_case("completed") {
                continue;
            }
            let id_str = job.id.to_string();
            let job_id = id_str.as_str();
            let name = job.name.clone().unwrap_or_else(|| format!("job {job_id}"));
            let conclusion = job.conclusion.as_deref().unwrap_or("");
            let run_title_ref = run_title.as_str();
            if !matches_ci(
                query,
                [job_id, name.as_str(), status, conclusion, run_title_ref],
            ) {
                continue;
            }
            let score = score_fields(query, [job_id, name.as_str(), run_title_ref]);

            let id = format!("{run_id}:{job_id}");
            let href = href("gh_job", &id);
            let subtitle = Some(format!("{run_title} · {status} {conclusion}"));

            upsert(
                merged,
                SearchResult {
                    kind: "gh_job".into(),
                    id,
                    title: name,
                    subtitle,
                    href,
                    score: 44.0 + score,
                    match_sources: vec!["operational".into()],
                },
            );
        }
    }
}

async fn list_runs(status: &str) -> Result<Vec<GhRun>, String> {
    let args: Vec<String> = vec![
        "run".into(),
        "list".into(),
        "--limit".into(),
        RUN_LIMIT.into(),
        "--status".into(),
        status.into(),
        "--json".into(),
        "databaseId,workflowName,displayTitle,status,conclusion,headBranch,event".into(),
    ];
    let out = run_gh(&args).await?;
    serde_json::from_slice(&out).map_err(|e| format!("invalid gh run list json: {e}"))
}

async fn run_view(run_id: i64) -> Result<GhRunView, String> {
    let args: Vec<String> = vec![
        "run".into(),
        "view".into(),
        run_id.to_string(),
        "--json".into(),
        "jobs,workflowName,displayTitle".into(),
    ];
    let out = run_gh(&args).await?;
    serde_json::from_slice(&out).map_err(|e| format!("invalid gh run view json: {e}"))
}

async fn run_gh(args: &[String]) -> Result<Vec<u8>, String> {
    let fut = Command::new("gh").args(args).output();
    let out = timeout(GH_TIMEOUT, fut)
        .await
        .map_err(|_| "gh timed out".to_string())
        .and_then(|res| res.map_err(|e| format!("could not spawn gh: {e}")))?;

    if !out.status.success() {
        let msg = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("gh exited {}: {msg}", out.status));
    }
    Ok(out.stdout)
}
