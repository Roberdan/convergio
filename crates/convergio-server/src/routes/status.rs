//! `/v1/status` — local dashboard summary for humans and agents.

use crate::app::AppState;
use crate::error::ApiError;
use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use convergio_durability::{
    Plan, PlanStatus, RecentCompletedTask, Task, TaskStatus, TelemetryCounters,
};
use serde::{Deserialize, Serialize};

/// Mount `/v1/status`.
pub fn router() -> Router<AppState> {
    Router::new().route("/v1/status", get(status))
}

#[derive(Deserialize)]
struct StatusQuery {
    #[serde(default = "default_plan_limit")]
    plan_limit: i64,
    #[serde(default = "default_completed_limit")]
    completed_limit: i64,
}

fn default_plan_limit() -> i64 {
    100
}

fn default_completed_limit() -> i64 {
    10
}

#[derive(Serialize)]
struct StatusResponse {
    active_plans: Vec<PlanSummary>,
    recent_completed_plans: Vec<PlanSummary>,
    recent_completed_tasks: Vec<CompletedTask>,
    /// Aggregate counters for the multi-agent dashboard. Each value
    /// is a single SQL aggregate; the block is additive so existing
    /// callers ignore it.
    telemetry: TelemetryCounters,
}

#[derive(Serialize)]
struct PlanSummary {
    id: String,
    title: String,
    description: Option<String>,
    project: Option<String>,
    status: PlanStatus,
    updated_at: String,
    tasks: TaskCounts,
    next_tasks: Vec<TaskSummary>,
}

#[derive(Default, Serialize)]
struct TaskCounts {
    total: usize,
    pending: usize,
    in_progress: usize,
    submitted: usize,
    done: usize,
    failed: usize,
}

#[derive(Serialize)]
struct TaskSummary {
    id: String,
    title: String,
    status: TaskStatus,
    agent_id: Option<String>,
    wave: i64,
    sequence: i64,
}

#[derive(Serialize)]
struct CompletedTask {
    id: String,
    title: String,
    plan_id: String,
    plan_title: String,
    project: Option<String>,
    updated_at: String,
}

async fn status(
    State(state): State<AppState>,
    Query(q): Query<StatusQuery>,
) -> Result<Json<StatusResponse>, ApiError> {
    // Negative caps mean "none"; the tasks side already uses
    // `.max(0)` below, so clamp once and reuse for both sides.
    let completed_cap = q.completed_limit.max(0) as usize;
    let plans = state.durability.plans().list(q.plan_limit).await?;
    let mut active_plans = Vec::new();
    let mut recent_completed_plans = Vec::new();

    for plan in plans {
        let summary = summarize_plan(&state, plan).await?;
        match summary.status {
            PlanStatus::Completed => {
                if recent_completed_plans.len() < completed_cap {
                    recent_completed_plans.push(summary);
                }
            }
            PlanStatus::Cancelled => {}
            PlanStatus::Draft | PlanStatus::Active => active_plans.push(summary),
        }
    }

    let recent_completed_tasks = state
        .durability
        .tasks()
        .list_recent_done(completed_cap as i64)
        .await?
        .into_iter()
        .map(completed_task)
        .collect();
    let telemetry = state.durability.telemetry().await?;
    Ok(Json(StatusResponse {
        active_plans,
        recent_completed_plans,
        recent_completed_tasks,
        telemetry,
    }))
}

async fn summarize_plan(state: &AppState, plan: Plan) -> Result<PlanSummary, ApiError> {
    let tasks = state.durability.tasks().list_by_plan(&plan.id).await?;
    let counts = task_counts(&tasks);
    let next_tasks = tasks
        .into_iter()
        .filter(|task| {
            matches!(
                task.status,
                TaskStatus::Pending | TaskStatus::InProgress | TaskStatus::Submitted
            )
        })
        .take(3)
        .map(task_summary)
        .collect();

    Ok(PlanSummary {
        id: plan.id,
        title: plan.title,
        description: plan.description,
        project: plan.project,
        status: plan.status,
        updated_at: plan.updated_at.to_rfc3339(),
        tasks: counts,
        next_tasks,
    })
}

fn task_counts(tasks: &[Task]) -> TaskCounts {
    let mut counts = TaskCounts {
        total: tasks.len(),
        ..TaskCounts::default()
    };
    for task in tasks {
        match task.status {
            TaskStatus::Pending => counts.pending += 1,
            TaskStatus::InProgress => counts.in_progress += 1,
            TaskStatus::Submitted => counts.submitted += 1,
            TaskStatus::Done => counts.done += 1,
            TaskStatus::Failed => counts.failed += 1,
        }
    }
    counts
}

fn task_summary(task: Task) -> TaskSummary {
    TaskSummary {
        id: task.id,
        title: task.title,
        status: task.status,
        agent_id: task.agent_id,
        wave: task.wave,
        sequence: task.sequence,
    }
}

fn completed_task(task: RecentCompletedTask) -> CompletedTask {
    CompletedTask {
        id: task.id,
        title: task.title,
        plan_id: task.plan_id,
        plan_title: task.plan_title,
        project: task.project,
        updated_at: task.updated_at.to_rfc3339(),
    }
}
