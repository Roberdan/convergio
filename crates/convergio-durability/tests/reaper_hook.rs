//! Regression test for the reaper's `on_reap` callback (issue #408).
//!
//! The server wires this hook to remove the reaped task's git
//! worktree from disk. Layer 1 stays git-agnostic — we only verify
//! here that the hook fires once per released task with the right id.

use chrono::{Duration, Utc};
use convergio_db::Pool;
use convergio_durability::reaper::{self, OnReap, ReaperConfig};
use convergio_durability::{init, Durability, NewPlan, NewTask, TaskStatus};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

#[tokio::test]
async fn on_reap_hook_fires_for_each_released_task() {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    let dur = Durability::new(pool);

    let plan = dur
        .create_plan(NewPlan {
            title: "hook test".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "hook me".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("agent-1"))
        .await
        .unwrap();

    let stale = (Utc::now() - Duration::seconds(3600)).to_rfc3339();
    sqlx::query("UPDATE tasks SET last_heartbeat_at = ? WHERE id = ?")
        .bind(&stale)
        .bind(&task.id)
        .execute(dur.pool().inner())
        .await
        .unwrap();

    let calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let calls_clone = calls.clone();
    let hook: OnReap = Arc::new(move |id: &str| {
        calls_clone.lock().unwrap().push(id.to_string());
    });

    let result = reaper::tick(
        &dur,
        &ReaperConfig {
            timeout: Duration::seconds(300),
            tick_interval: Duration::seconds(60),
            agent_reaper_enabled: false,
            agent_threshold: Duration::seconds(3600),
            on_reap: Some(hook),
        },
    )
    .await
    .unwrap();
    assert_eq!(result.tasks, 1);

    let observed = calls.lock().unwrap().clone();
    assert_eq!(observed, vec![task.id.clone()]);
}

#[tokio::test]
async fn missing_on_reap_does_not_break_release() {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    let dur = Durability::new(pool);

    let plan = dur
        .create_plan(NewPlan {
            title: "no hook".into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                wave: 1,
                sequence: 1,
                title: "no hook task".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("agent-x"))
        .await
        .unwrap();

    let stale = (Utc::now() - Duration::seconds(3600)).to_rfc3339();
    sqlx::query("UPDATE tasks SET last_heartbeat_at = ? WHERE id = ?")
        .bind(&stale)
        .bind(&task.id)
        .execute(dur.pool().inner())
        .await
        .unwrap();

    let result = reaper::tick(&dur, &ReaperConfig::default()).await.unwrap();
    assert_eq!(result.tasks, 1);
}
