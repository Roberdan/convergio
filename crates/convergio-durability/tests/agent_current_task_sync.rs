//! Agent ↔ task live-state sync (F46).
//!
//! Verifies that `Durability::transition_task` keeps
//! `agents.current_task_id` and `agents.status` in lock-step with the
//! task's `in_progress` lifecycle, so a `SELECT id, status,
//! current_task_id FROM agents` is a useful "who is working on what
//! now?" snapshot.

use convergio_db::Pool;
use convergio_durability::{init, Durability, NewAgent, NewPlan, NewTask, TaskStatus};
use serde_json::json;
use tempfile::TempDir;

async fn fresh() -> (Durability, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool: Pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

fn agent(id: &str) -> NewAgent {
    NewAgent {
        id: id.into(),
        kind: "claude-code".into(),
        name: None,
        host: None,
        capabilities: vec![],
        metadata: json!({}),
    }
}

async fn make_task(dur: &Durability, plan_title: &str) -> String {
    let plan = dur
        .create_plan(NewPlan {
            title: plan_title.into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    dur.create_task(
        &plan.id,
        NewTask {
            wave: 1,
            sequence: 1,
            title: "t".into(),
            description: None,
            evidence_required: vec![],
            runner_kind: None,
            profile: None,
            max_budget_usd: None,
        },
    )
    .await
    .unwrap()
    .id
}

#[tokio::test]
async fn claim_marks_agent_working_and_points_at_task() {
    let (dur, _dir) = fresh().await;
    dur.register_agent(agent("alpha")).await.unwrap();
    let task_id = make_task(&dur, "p1").await;

    dur.transition_task(&task_id, TaskStatus::InProgress, Some("alpha"))
        .await
        .unwrap();

    let a = dur.agents().get("alpha").await.unwrap();
    assert_eq!(a.status, "working");
    assert_eq!(a.current_task_id.as_deref(), Some(task_id.as_str()));
}

#[tokio::test]
async fn release_clears_current_task_and_marks_idle() {
    let (dur, _dir) = fresh().await;
    dur.register_agent(agent("beta")).await.unwrap();
    let task_id = make_task(&dur, "p2").await;

    dur.transition_task(&task_id, TaskStatus::InProgress, Some("beta"))
        .await
        .unwrap();
    dur.transition_task(&task_id, TaskStatus::Submitted, Some("beta"))
        .await
        .unwrap();

    let b = dur.agents().get("beta").await.unwrap();
    assert_eq!(b.status, "idle");
    assert!(b.current_task_id.is_none());
}

#[tokio::test]
async fn release_does_not_disturb_agent_already_on_a_different_task() {
    // Agent claims task A, then claims task B (live-state now points
    // at B). Releasing A must NOT clear the agent — current_task_id
    // is still B and status still 'working'.
    let (dur, _dir) = fresh().await;
    dur.register_agent(agent("gamma")).await.unwrap();
    let a_id = make_task(&dur, "pa").await;
    let b_id = make_task(&dur, "pb").await;

    dur.transition_task(&a_id, TaskStatus::InProgress, Some("gamma"))
        .await
        .unwrap();
    dur.transition_task(&b_id, TaskStatus::InProgress, Some("gamma"))
        .await
        .unwrap();
    dur.transition_task(&a_id, TaskStatus::Submitted, Some("gamma"))
        .await
        .unwrap();

    let g = dur.agents().get("gamma").await.unwrap();
    assert_eq!(g.status, "working");
    assert_eq!(g.current_task_id.as_deref(), Some(b_id.as_str()));
}

#[tokio::test]
async fn unregistered_agent_does_not_error_on_claim() {
    // Manual-mode agents that never called `register_agent` (e.g. a
    // shell script driving `cvg task transition` directly) still
    // succeed — the agents UPDATE just affects zero rows.
    let (dur, _dir) = fresh().await;
    let task_id = make_task(&dur, "p3").await;

    dur.transition_task(&task_id, TaskStatus::InProgress, Some("ghost"))
        .await
        .unwrap();

    assert!(dur.agents().list().await.unwrap().is_empty());
}
