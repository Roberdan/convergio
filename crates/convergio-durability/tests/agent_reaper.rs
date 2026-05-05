//! Agent-reaper integration tests — drives `tick` and
//! `retire_stale_agents` directly without wall-clock waits.

use chrono::{Duration, Utc};
use convergio_db::Pool;
use convergio_durability::reaper::{self, ReaperConfig};
use convergio_durability::{init, AgentHeartbeat, Durability, NewAgent};
use tempfile::tempdir;

async fn fresh_durability() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

fn new_agent(id: &str) -> NewAgent {
    NewAgent {
        id: id.into(),
        kind: "claude".into(),
        name: Some("test agent".into()),
        host: None,
        capabilities: vec![],
        metadata: serde_json::json!({}),
    }
}

#[tokio::test]
async fn agent_reaper_retires_stale_agents() {
    let (dur, _dir) = fresh_durability().await;

    dur.register_agent(new_agent("stale-agent-1"))
        .await
        .unwrap();
    dur.register_agent(new_agent("stale-agent-2"))
        .await
        .unwrap();

    // Back-date heartbeats so they look 2h stale.
    let stale_ts = (Utc::now() - Duration::seconds(7200)).to_rfc3339();
    sqlx::query(
        "UPDATE agents SET last_heartbeat_at = ? WHERE id IN ('stale-agent-1', 'stale-agent-2')",
    )
    .bind(&stale_ts)
    .execute(dur.pool().inner())
    .await
    .unwrap();

    let result = reaper::tick(
        &dur,
        &ReaperConfig {
            timeout: Duration::seconds(300),
            tick_interval: Duration::seconds(60),
            agent_reaper_enabled: true,
            agent_threshold: Duration::seconds(3600),
        },
    )
    .await
    .unwrap();

    assert_eq!(result.agents, 2);

    // Both agents must now be terminated.
    let a1 = dur.agents().get("stale-agent-1").await.unwrap();
    let a2 = dur.agents().get("stale-agent-2").await.unwrap();
    assert_eq!(a1.status, "terminated");
    assert_eq!(a2.status, "terminated");

    // Audit chain must include agent.retired_stale and still verify clean.
    let report = dur.audit().verify(None, None).await.unwrap();
    assert!(
        report.ok,
        "audit chain broken after agent reaper: {report:?}"
    );

    // Confirm audit events contain agent.retired_stale rows.
    let entries: Vec<String> = sqlx::query_scalar(
        "SELECT transition FROM audit_log WHERE transition = 'agent.retired_stale'",
    )
    .fetch_all(dur.pool().inner())
    .await
    .unwrap();
    assert_eq!(entries.len(), 2);
}

#[tokio::test]
async fn agent_reaper_skips_fresh_agents() {
    let (dur, _dir) = fresh_durability().await;

    dur.register_agent(new_agent("fresh-agent")).await.unwrap();
    // Heartbeat just recorded — well within threshold.
    dur.heartbeat_agent(
        "fresh-agent",
        AgentHeartbeat {
            current_task_id: None,
            status: None,
        },
    )
    .await
    .unwrap();

    let result = reaper::tick(
        &dur,
        &ReaperConfig {
            timeout: Duration::seconds(300),
            tick_interval: Duration::seconds(60),
            agent_reaper_enabled: true,
            agent_threshold: Duration::seconds(3600),
        },
    )
    .await
    .unwrap();

    assert_eq!(result.agents, 0);
    let agent = dur.agents().get("fresh-agent").await.unwrap();
    assert_ne!(agent.status, "terminated");
}

#[tokio::test]
async fn agent_reaper_disabled_does_not_retire_stale_agents() {
    let (dur, _dir) = fresh_durability().await;

    dur.register_agent(new_agent("would-be-stale"))
        .await
        .unwrap();
    let stale_ts = (Utc::now() - Duration::seconds(7200)).to_rfc3339();
    sqlx::query("UPDATE agents SET last_heartbeat_at = ? WHERE id = 'would-be-stale'")
        .bind(&stale_ts)
        .execute(dur.pool().inner())
        .await
        .unwrap();

    let result = reaper::tick(
        &dur,
        &ReaperConfig {
            timeout: Duration::seconds(300),
            tick_interval: Duration::seconds(60),
            agent_reaper_enabled: false,
            agent_threshold: Duration::seconds(3600),
        },
    )
    .await
    .unwrap();

    assert_eq!(result.agents, 0);
    // Agent must still be alive.
    let agent = dur.agents().get("would-be-stale").await.unwrap();
    assert_ne!(agent.status, "terminated");
}

#[tokio::test]
async fn retire_stale_agents_dry_run_does_not_mutate() {
    let (dur, _dir) = fresh_durability().await;

    dur.register_agent(new_agent("dry-run-agent"))
        .await
        .unwrap();
    let stale_ts = (Utc::now() - Duration::seconds(7200)).to_rfc3339();
    sqlx::query("UPDATE agents SET last_heartbeat_at = ? WHERE id = 'dry-run-agent'")
        .bind(&stale_ts)
        .execute(dur.pool().inner())
        .await
        .unwrap();

    let result = dur.retire_stale_agents(3600, true).await.unwrap();
    assert!(!result.applied);
    assert_eq!(result.agents.len(), 1);
    assert!(!result.agents[0].retired);

    // Agent must still be alive.
    let agent = dur.agents().get("dry-run-agent").await.unwrap();
    assert_ne!(agent.status, "terminated");
}
