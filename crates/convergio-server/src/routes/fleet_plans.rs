//! Fleet plan routes (ADR-0038, F3-2 / F3-3).
//!
//! Routes:
//! - `POST   /v1/fleet/plans`                                — create
//! - `GET    /v1/fleet/plans`                                — list
//! - `GET    /v1/fleet/plans/:id`                            — show + links
//! - `POST   /v1/fleet/plans/:id/repos`                      — link a repo
//! - `POST   /v1/fleet/plans/:id/repos/:repo/tasks`          — add per-repo task
//! - `POST   /v1/fleet/plans/:id/validate`                   — cross-repo gate verdict (F3-3)
//!
//! Status rollup is derived in `show` from the durability layer — no
//! aggregate column on `fleet_plans`. The link insertion is
//! idempotent on `(fleet_plan_id, repo)`.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_durability::{Durability, NewTask};
use convergio_fleet::{FleetError, FleetPlanRepoLink, FleetPlanView, NewFleetPlan};
use convergio_thor::{Thor, Verdict};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

/// Mount the fleet-plan routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/fleet/plans", post(create).get(list))
        .route("/v1/fleet/plans/:id", get(show))
        .route("/v1/fleet/plans/:id/repos", post(link_repo))
        .route("/v1/fleet/plans/:id/repos/:repo/tasks", post(add_task))
        .route("/v1/fleet/plans/:id/validate", post(validate))
}

async fn create(
    State(state): State<AppState>,
    Json(input): Json<NewFleetPlan>,
) -> Result<Json<Value>, ApiError> {
    let plan = state.fleet_plans.create(input).await?;
    Ok(Json(json!(plan)))
}

#[derive(Deserialize)]
struct ListQuery {
    scope: Option<String>,
}

async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<convergio_fleet::FleetPlan>>, ApiError> {
    let plans = state.fleet_plans.list(q.scope.as_deref()).await?;
    Ok(Json(plans))
}

async fn show(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<FleetPlanView>, ApiError> {
    Ok(Json(state.fleet_plans.show(&id).await?))
}

#[derive(Deserialize)]
struct LinkRepoInput {
    /// Repo name (matches `fleet_repos.name`).
    repo: String,
    /// Per-repo plan id in `convergio-durability`.
    repo_plan_id: String,
}

async fn link_repo(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(input): Json<LinkRepoInput>,
) -> Result<Json<Value>, ApiError> {
    // The fleet plan must exist; surface 404 here rather than letting
    // the link table accept a parent-less link (no FK to fleet_plans
    // on the row schema; the integrity check lives at this layer).
    state.fleet_plans.get(&id).await?;
    // The per-repo plan must exist in convergio-durability. Without
    // this check a typo would produce a dangling link that only
    // surfaces later when `add-task` tries to create the task on a
    // non-existent plan. fleet_plan_repos has no FK to plans, so
    // validate here.
    state.durability.plans().get(&input.repo_plan_id).await?;
    let link = FleetPlanRepoLink {
        fleet_plan_id: id,
        repo: input.repo,
        repo_plan_id: input.repo_plan_id,
    };
    state.fleet_plans.link_repo(&link).await?;
    Ok(Json(json!(link)))
}

#[derive(Deserialize)]
struct AddTaskInput {
    /// Task title.
    title: String,
    /// Optional task description.
    #[serde(default)]
    description: Option<String>,
    /// Wave (defaults to 1).
    #[serde(default = "default_wave")]
    wave: i64,
    /// Sequence within the wave (defaults to next).
    #[serde(default = "default_seq")]
    sequence: i64,
    /// Evidence kinds required for the task to submit.
    #[serde(default)]
    evidence_required: Vec<String>,
}

fn default_wave() -> i64 {
    1
}
fn default_seq() -> i64 {
    1
}

async fn add_task(
    State(state): State<AppState>,
    Path((id, repo)): Path<(String, String)>,
    Json(input): Json<AddTaskInput>,
) -> Result<Json<Value>, ApiError> {
    // Resolve the per-repo plan id from the fleet-plan links.
    let links = state.fleet_plans.links(&id).await?;
    let link = links.iter().find(|l| l.repo == repo).ok_or_else(|| {
        ApiError::Fleet(FleetError::NotFound(format!(
            "repo '{repo}' not linked to fleet plan '{id}'"
        )))
    })?;
    let task = state
        .durability
        .create_task(
            &link.repo_plan_id,
            NewTask {
                title: input.title,
                description: input.description,
                wave: input.wave,
                sequence: input.sequence,
                evidence_required: input.evidence_required,
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await?;
    Ok(Json(json!({
        "fleet_plan_id": id,
        "repo": repo,
        "repo_plan_id": link.repo_plan_id,
        "task": task,
    })))
}

/// Per-repo verdict in a fleet-validate response (F3-3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoVerdict {
    /// Repo name (matches `fleet_repos.name`).
    pub repo: String,
    /// Per-repo plan id under that repo's durability state.
    pub repo_plan_id: String,
    /// Inner Thor verdict — `pass` or `fail` with reasons. The
    /// special variant `timeout` is emitted when the gate exceeds
    /// the configured per-repo timeout.
    #[serde(flatten)]
    pub verdict: RepoOutcome,
}

/// Discriminated outcome shape for a single repo.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum RepoOutcome {
    /// All gates passed.
    Pass,
    /// One or more gates refused (or task state failed).
    Fail {
        /// One reason per failing task / missing evidence / pipeline.
        reasons: Vec<String>,
    },
    /// Thor::validate did not return within the per-repo timeout.
    Timeout {
        /// The deadline in seconds (used by the operator's CLI).
        secs: u64,
    },
}

/// Aggregated response from `POST /v1/fleet/plans/:id/validate`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetValidateReport {
    /// Fleet plan id this report concerns.
    pub fleet_plan_id: String,
    /// `true` iff every linked repo's verdict is `pass`. Always 200
    /// on the wire — the client reads this field, not the status.
    pub passing: bool,
    /// One entry per linked repo, in stable repo-name order.
    pub verdicts: Vec<RepoVerdict>,
}

/// Default per-repo timeout (seconds) — see F3-3 design decisions.
pub(crate) const DEFAULT_PER_REPO_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Default, Deserialize)]
struct ValidateQuery {
    per_repo_timeout_secs: Option<u64>,
}

async fn validate(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<ValidateQuery>,
) -> Result<Json<FleetValidateReport>, ApiError> {
    // Resolve the fleet plan (404 if missing) and gather its links.
    state.fleet_plans.get(&id).await?;
    let mut links = state.fleet_plans.links(&id).await?;
    links.sort_by(|a, b| a.repo.cmp(&b.repo)); // stable ordering for callers
    let timeout_secs = q
        .per_repo_timeout_secs
        .unwrap_or(DEFAULT_PER_REPO_TIMEOUT_SECS)
        .max(1);
    let timeout = Duration::from_secs(timeout_secs);

    // Spawn per-repo validation in parallel. Thor::validate is
    // I/O-bound (durability calls) so latency is max(per-repo), not
    // sum. Each branch is wrapped in tokio::time::timeout so a stuck
    // gate cannot block the aggregate result.
    //
    // Collect the JoinHandles eagerly with `.collect::<Vec<_>>()`
    // BEFORE awaiting any of them — otherwise the lazy `.map()`
    // iterator only spawns a worker when we pull the next item, and
    // the `for h in handles { h.await }` loop would serialise the
    // whole thing into spawn-await-spawn-await with latency
    // = sum(per-repo) instead of max(per-repo). Codex P1 on #375.
    let durability: Durability = (*state.durability).clone();
    let handles: Vec<_> = links
        .into_iter()
        .map(|link| {
            let dur = durability.clone();
            let secs = timeout_secs;
            tokio::spawn(async move {
                let thor = Thor::new(dur);
                // Read-only: cross-repo validate is "report only". A
                // pass here doesn't promote tasks; that stays per-plan
                // via `cvg plan validate` so multi-repo state cannot
                // half-promote. See ADR-0038 F3-3 decisions.
                let outcome =
                    match tokio::time::timeout(timeout, thor.dry_run(&link.repo_plan_id)).await {
                        Ok(Ok(Verdict::Pass)) => RepoOutcome::Pass,
                        Ok(Ok(Verdict::Fail { reasons })) => RepoOutcome::Fail { reasons },
                        Ok(Err(e)) => RepoOutcome::Fail {
                            reasons: vec![format!("thor error: {e}")],
                        },
                        Err(_) => RepoOutcome::Timeout { secs },
                    };
                RepoVerdict {
                    repo: link.repo,
                    repo_plan_id: link.repo_plan_id,
                    verdict: outcome,
                }
            })
        })
        .collect();
    let mut verdicts = Vec::new();
    for h in handles {
        match h.await {
            Ok(v) => verdicts.push(v),
            Err(join_err) => {
                return Err(ApiError::Internal(format!(
                    "fleet validate worker panicked: {join_err}"
                )));
            }
        }
    }
    let passing = verdicts
        .iter()
        .all(|v| matches!(v.verdict, RepoOutcome::Pass));
    Ok(Json(FleetValidateReport {
        fleet_plan_id: id,
        passing,
        verdicts,
    }))
}
