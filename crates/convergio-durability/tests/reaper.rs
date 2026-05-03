//! Reaper integration test — drives `tick` directly so it doesn't have
//! to wait on real wall-clock time.

use chrono::{Duration, Utc};
use convergio_db::Pool;
use convergio_durability::reaper::{self, ReaperConfig};
use convergio_durability::{init, Durability, NewPlan, NewTask, TaskStatus};
use sqlx::Row;
use tempfile::tempdir;

async fn fresh_durability() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

#[tokio::test]
async fn task_reaper_indexes_migration_applies() {
    let (dur, _dir) = fresh_durability().await;

    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM pragma_index_list('tasks') \
         WHERE name IN ('idx_tasks_reaper_heartbeat', 'idx_tasks_reaper_no_heartbeat') \
         ORDER BY name",
    )
    .fetch_all(dur.pool().inner())
    .await
    .unwrap();
    assert_eq!(
        names,
        vec![
            "idx_tasks_reaper_heartbeat".to_string(),
            "idx_tasks_reaper_no_heartbeat".to_string()
        ]
    );

    let applied: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 8")
            .fetch_one(dur.pool().inner())
            .await
            .unwrap();
    assert_eq!(applied, 1);
}

#[tokio::test]
async fn stale_scan_query_uses_reaper_indexes() {
    let (dur, _dir) = fresh_durability().await;

    let plan = sqlx::query(
        "EXPLAIN QUERY PLAN \
         SELECT id, agent_id FROM tasks \
         WHERE status = 'in_progress' \
           AND last_heartbeat_at < ? \
         UNION ALL \
         SELECT id, agent_id FROM tasks \
         WHERE status = 'in_progress' \
           AND last_heartbeat_at IS NULL \
           AND updated_at < ?",
    )
    .bind("2026-01-01T00:00:00Z")
    .bind("2026-01-01T00:00:00Z")
    .fetch_all(dur.pool().inner())
    .await
    .unwrap();

    let details = plan
        .iter()
        .map(|row| row.get::<String, _>("detail"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(details.contains("idx_tasks_reaper_heartbeat"), "{details}");
    assert!(
        details.contains("idx_tasks_reaper_no_heartbeat"),
        "{details}"
    );
}

#[tokio::test]
async fn reaps_tasks_with_stale_heartbeat() {
    let (dur, _dir) = fresh_durability().await;

    let plan = dur
        .create_plan(NewPlan {
            title: "reaper test".into(),
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
                title: "stuck task".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    // Claim it as agent-1
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("agent-1"))
        .await
        .unwrap();

    // Forge a stale heartbeat by writing it back-dated.
    let stale = (Utc::now() - Duration::seconds(3600)).to_rfc3339();
    sqlx::query("UPDATE tasks SET last_heartbeat_at = ? WHERE id = ?")
        .bind(&stale)
        .bind(&task.id)
        .execute(dur.pool().inner())
        .await
        .unwrap();

    // Tick with a 5-minute timeout — task is 1h stale, must be reaped.
    let n = reaper::tick(
        &dur,
        &ReaperConfig {
            timeout: Duration::seconds(300),
            tick_interval: Duration::seconds(60),
        },
    )
    .await
    .unwrap();
    assert_eq!(n, 1);

    let after = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::Pending);
    assert!(after.agent_id.is_none());

    // Audit chain must include task.reaped and still verify clean.
    let report = dur.audit().verify(None, None).await.unwrap();
    assert!(report.ok);
}

#[tokio::test]
async fn does_not_reap_fresh_tasks() {
    let (dur, _dir) = fresh_durability().await;

    let plan = dur
        .create_plan(NewPlan {
            title: "fresh".into(),
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
                title: "fresh task".into(),
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
    dur.tasks().heartbeat(&task.id).await.unwrap();

    let n = reaper::tick(
        &dur,
        &ReaperConfig {
            timeout: Duration::seconds(60),
            tick_interval: Duration::seconds(30),
        },
    )
    .await
    .unwrap();
    assert_eq!(n, 0);

    let after = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::InProgress);
}

#[tokio::test]
async fn reaps_tasks_that_never_heartbeat() {
    let (dur, _dir) = fresh_durability().await;

    let plan = dur
        .create_plan(NewPlan {
            title: "never heartbeat".into(),
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
                title: "claimed then died".into(),
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
    sqlx::query("UPDATE tasks SET updated_at = ?, last_heartbeat_at = NULL WHERE id = ?")
        .bind(&stale)
        .bind(&task.id)
        .execute(dur.pool().inner())
        .await
        .unwrap();

    let n = reaper::tick(
        &dur,
        &ReaperConfig {
            timeout: Duration::seconds(300),
            tick_interval: Duration::seconds(60),
        },
    )
    .await
    .unwrap();
    assert_eq!(n, 1);

    let after = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::Pending);
    assert!(after.agent_id.is_none());
}
