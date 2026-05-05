//! Reaper integration test — drives `tick` directly. Covers task pass
//! and agent staleness pass (P0-3, ref 544e78cc).

use chrono::{Duration, Utc};
use convergio_db::Pool;
use convergio_durability::reaper::{self, ReaperConfig};
use convergio_durability::{init, Durability, NewAgent, NewPlan, NewTask, Task, TaskStatus};
use sqlx::Row;
use tempfile::tempdir;

async fn fresh_durability() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}
async fn mk_task(dur: &Durability, title: &str) -> Task {
    let plan = dur
        .create_plan(NewPlan {
            title: title.into(),
            description: None,
            project: None,
        })
        .await
        .unwrap();
    let nt = NewTask {
        wave: 1,
        sequence: 1,
        title: title.into(),
        description: None,
        evidence_required: vec![],
        runner_kind: None,
        profile: None,
        max_budget_usd: None,
    };
    dur.create_task(&plan.id, nt).await.unwrap()
}
fn cfg(timeout_secs: i64, agent_threshold: i64) -> ReaperConfig {
    ReaperConfig {
        timeout: Duration::seconds(timeout_secs),
        tick_interval: Duration::seconds(60),
        agent_threshold: Duration::seconds(agent_threshold),
    }
}
async fn mk_agent(dur: &Durability, id: &str, hb_age_secs: i64) {
    dur.register_agent(NewAgent {
        id: id.into(),
        kind: "subagent".into(),
        name: None,
        host: None,
        capabilities: vec![],
        metadata: serde_json::json!({}),
    })
    .await
    .unwrap();
    let ts = (Utc::now() - Duration::seconds(hb_age_secs)).to_rfc3339();
    sqlx::query("UPDATE agents SET status='working', last_heartbeat_at=?, updated_at=? WHERE id=?")
        .bind(&ts)
        .bind(&ts)
        .bind(id)
        .execute(dur.pool().inner())
        .await
        .unwrap();
}

#[tokio::test]
async fn task_reaper_indexes_migration_applies() {
    let (dur, _d) = fresh_durability().await;
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
            "idx_tasks_reaper_heartbeat",
            "idx_tasks_reaper_no_heartbeat"
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
    let (dur, _d) = fresh_durability().await;
    let plan = sqlx::query(
        "EXPLAIN QUERY PLAN \
         SELECT id, agent_id FROM tasks \
         WHERE status = 'in_progress' AND last_heartbeat_at < ? \
         UNION ALL \
         SELECT id, agent_id FROM tasks \
         WHERE status = 'in_progress' AND last_heartbeat_at IS NULL AND updated_at < ?",
    )
    .bind("2026-01-01T00:00:00Z")
    .bind("2026-01-01T00:00:00Z")
    .fetch_all(dur.pool().inner())
    .await
    .unwrap();
    let d: String = plan
        .iter()
        .map(|row| row.get::<String, _>("detail"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(d.contains("idx_tasks_reaper_heartbeat"), "{d}");
    assert!(d.contains("idx_tasks_reaper_no_heartbeat"), "{d}");
}

#[tokio::test]
async fn reaps_tasks_with_stale_heartbeat() {
    let (dur, _d) = fresh_durability().await;
    let task = mk_task(&dur, "stuck task").await;
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

    let (tasks, _) = reaper::tick(&dur, &cfg(300, 0)).await.unwrap();
    assert_eq!(tasks, 1);
    let after = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::Pending);
    assert!(after.agent_id.is_none());
    assert!(dur.audit().verify(None, None).await.unwrap().ok);
}

#[tokio::test]
async fn does_not_reap_fresh_tasks() {
    let (dur, _d) = fresh_durability().await;
    let task = mk_task(&dur, "fresh task").await;
    dur.transition_task(&task.id, TaskStatus::InProgress, Some("agent-1"))
        .await
        .unwrap();
    dur.tasks().heartbeat(&task.id).await.unwrap();

    let (tasks, _) = reaper::tick(&dur, &cfg(60, 0)).await.unwrap();
    assert_eq!(tasks, 0);
    assert_eq!(
        dur.tasks().get(&task.id).await.unwrap().status,
        TaskStatus::InProgress
    );
}

#[tokio::test]
async fn reaps_tasks_that_never_heartbeat() {
    let (dur, _d) = fresh_durability().await;
    let task = mk_task(&dur, "claimed then died").await;
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

    let (tasks, _) = reaper::tick(&dur, &cfg(300, 0)).await.unwrap();
    assert_eq!(tasks, 1);
    let after = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(after.status, TaskStatus::Pending);
    assert!(after.agent_id.is_none());
}

// Agent staleness pass (P0-3, ref 544e78cc): hb ages 0/30m/2h — only
// the 2h agent retires, with `agent.retired` + `agent.retired_stale`.
#[tokio::test]
async fn agent_pass_retires_only_stale_and_writes_audit_pair() {
    let (dur, _d) = fresh_durability().await;
    mk_agent(&dur, "fresh", 0).await;
    mk_agent(&dur, "midway", 30 * 60).await;
    mk_agent(&dur, "stale", 2 * 3600).await;

    let (tasks, agents) = reaper::tick(&dur, &cfg(300, 3600)).await.unwrap();
    assert_eq!((tasks, agents), (0, 1));
    for id in ["fresh", "midway", "stale"] {
        let st = dur.agents().get(id).await.unwrap().status;
        assert_eq!(st == "terminated", id == "stale", "{id}: {st}");
    }

    let audit = dur
        .agents()
        .recent_audit_for_agent("stale", 10)
        .await
        .unwrap();
    let kinds: Vec<&str> = audit.iter().map(|a| a.transition.as_str()).collect();
    assert!(kinds.contains(&"agent.retired"), "{kinds:?}");
    assert!(kinds.contains(&"agent.retired_stale"), "{kinds:?}");
    let row = audit
        .iter()
        .find(|a| a.transition == "agent.retired_stale")
        .unwrap();
    assert_eq!(row.payload["agent_id"], "stale");
    assert_eq!(row.payload["threshold_seconds"], 3600);
    assert_eq!(row.payload["reason"], "stale_heartbeat");
    assert!(row.payload["last_heartbeat_at"].is_string());
    assert!(dur.audit().verify(None, None).await.unwrap().ok);
}

// `agent_threshold = 0` disables the agent pass; task pass still runs.
#[tokio::test]
async fn agent_pass_disabled_when_threshold_zero() {
    let (dur, _d) = fresh_durability().await;
    mk_agent(&dur, "ancient", 24 * 3600).await;
    let (_, agents) = reaper::tick(&dur, &cfg(300, 0)).await.unwrap();
    assert_eq!(agents, 0);
    let st = dur.agents().get("ancient").await.unwrap().status;
    assert_ne!(st, "terminated");
}

// Already-terminated agents are skipped (no double-retire / dup audit).
#[tokio::test]
async fn agent_pass_skips_already_terminated() {
    let (dur, _d) = fresh_durability().await;
    mk_agent(&dur, "done", 4 * 3600).await;
    dur.retire_agent("done").await.unwrap();
    let (_, agents) = reaper::tick(&dur, &cfg(300, 3600)).await.unwrap();
    assert_eq!(agents, 0);
}
