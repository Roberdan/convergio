//! `agent.session_started` audit row dedup behaviour.
//!
//! Forces the SessionStart hook contract into a regression test:
//! - first registration emits `agent.session_started`,
//! - re-registration within the dedup window does NOT emit a second
//!   row (so a Claude Code session that resumes does not spam),
//! - registration after a stale heartbeat (>30 min) emits a fresh
//!   row (a real new shell, not a transient context restore).

use convergio_db::Pool;
use convergio_durability::{init, AgentHeartbeat, Durability, NewAgent};
use serde_json::json;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}", db.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

fn input(id: &str) -> NewAgent {
    NewAgent {
        id: id.into(),
        kind: "claude".into(),
        name: Some("Claude Code".into()),
        host: Some("macbook".into()),
        capabilities: vec!["code".into()],
        metadata: json!({"repo_root": "/tmp/repo"}),
    }
}

async fn count_session_started(dur: &Durability, agent_id: &str) -> usize {
    let pool = dur.pool().inner();
    let n: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM audit_log \
         WHERE transition = 'agent.session_started' AND entity_id = ?",
    )
    .bind(agent_id)
    .fetch_one(pool)
    .await
    .unwrap();
    n as usize
}

#[tokio::test]
async fn first_register_emits_session_started_and_dedups_on_repeat() {
    let (dur, _dir) = fresh().await;
    dur.register_agent(input("claude-code-alice"))
        .await
        .unwrap();
    assert_eq!(count_session_started(&dur, "claude-code-alice").await, 1);

    // Heartbeat then re-register. Within the dedup window (30 min)
    // this MUST NOT emit a new `agent.session_started`. The audit
    // chain still has the registration row + the heartbeat row.
    dur.heartbeat_agent(
        "claude-code-alice",
        AgentHeartbeat {
            current_task_id: None,
            status: Some("idle".into()),
        },
    )
    .await
    .unwrap();
    dur.register_agent(input("claude-code-alice"))
        .await
        .unwrap();
    assert_eq!(count_session_started(&dur, "claude-code-alice").await, 1);

    // Force the heartbeat into the past (>30 min) and register
    // again. This represents a fresh shell after a long pause and
    // MUST emit a new session-started row.
    let pool = dur.pool().inner();
    let stale = (chrono::Utc::now() - chrono::Duration::hours(2)).to_rfc3339();
    sqlx::query("UPDATE agents SET last_heartbeat_at = ? WHERE id = ?")
        .bind(&stale)
        .bind("claude-code-alice")
        .execute(pool)
        .await
        .unwrap();
    dur.register_agent(input("claude-code-alice"))
        .await
        .unwrap();
    assert_eq!(count_session_started(&dur, "claude-code-alice").await, 2);

    // Audit chain stays intact across all of this.
    assert!(dur.audit().verify(None, None).await.unwrap().ok);
}
