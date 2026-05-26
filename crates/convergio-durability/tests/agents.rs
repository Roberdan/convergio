//! Agent registry tests.

use convergio_db::Pool;
use convergio_durability::{
    init, AgentHeartbeat, Durability, DurabilityError, NewAgent, NewPlan, NewTask, TaskStatus,
};
use serde_json::json;
use tempfile::TempDir;

async fn fresh() -> (Durability, TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}", db.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

fn new_agent(id: &str) -> NewAgent {
    NewAgent {
        id: id.into(),
        kind: "copilot".into(),
        name: Some("Copilot worker".into()),
        host: Some("terminal".into()),
        capabilities: vec!["code".into(), "test".into()],
        metadata: json!({"pid": 123}),
    }
}

#[tokio::test]
async fn register_is_idempotent_and_heartbeat_updates_status() {
    let (dur, _dir) = fresh().await;
    let agent = dur.register_agent(new_agent("agent-a")).await.unwrap();
    assert_eq!(agent.status, "idle");
    assert_eq!(agent.capabilities, ["code", "test"]);

    let agent = dur.register_agent(new_agent("agent-a")).await.unwrap();
    assert_eq!(agent.id, "agent-a");
    let agent = dur
        .heartbeat_agent(
            "agent-a",
            AgentHeartbeat {
                current_task_id: Some("task-1".into()),
                status: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(agent.status, "working");
    assert_eq!(agent.current_task_id.as_deref(), Some("task-1"));

    let listed = dur.agents().list().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert!(dur.audit().verify(None, None).await.unwrap().ok);
}

#[tokio::test]
async fn invalid_and_unknown_agents_are_rejected() {
    let (dur, _dir) = fresh().await;
    let err = dur.register_agent(new_agent("bad id")).await.unwrap_err();
    assert!(matches!(err, DurabilityError::InvalidAgent { .. }));

    let err = dur
        .heartbeat_agent(
            "missing",
            AgentHeartbeat {
                current_task_id: None,
                status: None,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, DurabilityError::NotFound { entity, .. } if entity == "agent"));
}

#[tokio::test]
async fn retire_marks_agent_retired() {
    let (dur, _dir) = fresh().await;
    dur.register_agent(new_agent("agent-a")).await.unwrap();
    let agent = dur.retire_agent("agent-a").await.unwrap();
    assert_eq!(agent.status, "retired");
    assert!(agent.current_task_id.is_none());
}

// kind validation (closes the v0.2 friction-task on NewAgent.kind enum
// hygiene; see commit message and friction log F49).

#[tokio::test]
async fn register_accepts_canonical_kinds() {
    let (dur, _dir) = fresh().await;
    for kind in ["claude", "copilot", "cursor", "shell", "codex", "aider"] {
        let mut a = new_agent(&format!("a-{kind}"));
        a.kind = kind.into();
        dur.register_agent(a).await.unwrap();
    }
}

// `subagent` is the documented kind for Claude Code subagents
// spawned via the parent's Task tool (see /cvg-spawn skill and
// docs/multi-agent-operating-model.md § Subagent lifecycle).
// Validation must accept it as a first-class kind.
#[tokio::test]
async fn register_accepts_subagent_kind() {
    let (dur, _dir) = fresh().await;
    let mut a = new_agent("subagent-demo-deadbeef");
    a.kind = "subagent".into();
    let agent = dur
        .register_agent(a)
        .await
        .expect("kind 'subagent' must be accepted");
    assert_eq!(agent.kind, "subagent");
}

#[tokio::test]
async fn register_merges_metadata_instead_of_overwriting() {
    let (dur, _dir) = fresh().await;
    let mut a = new_agent("agent-a");
    a.metadata = json!({"usage": {"input_tokens": 10, "output_tokens": 20, "cost_usd": 0.5}});
    dur.register_agent(a).await.unwrap();

    let mut b = new_agent("agent-a");
    b.metadata = json!({"runner": "claude-shell-wrapper"});
    let agent = dur.register_agent(b).await.unwrap();
    assert_eq!(agent.metadata["runner"], "claude-shell-wrapper");
    assert_eq!(agent.metadata["usage"]["input_tokens"], 10);
    assert_eq!(agent.metadata["usage"]["output_tokens"], 20);
}

#[tokio::test]
async fn usage_evidence_aggregates_into_agent_metadata() {
    let (dur, _dir) = fresh().await;
    dur.register_agent(new_agent("agent-a")).await.unwrap();

    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
            description: None,
            project: None,
            no_dispatch_default: false,
        })
        .await
        .unwrap();
    let task = dur
        .create_task(
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
                no_dispatch: None,
            },
        )
        .await
        .unwrap();

    dur.transition_task(&task.id, TaskStatus::InProgress, Some("agent-a"))
        .await
        .unwrap();

    dur.attach_evidence(
        &task.id,
        "usage",
        json!({
            "input_tokens": 5,
            "output_tokens": 7,
            "model": "claude:opus",
            "cost_usd": 0.01
        }),
        None,
    )
    .await
    .unwrap();

    dur.attach_evidence(
        &task.id,
        "usage",
        json!({
            "input_tokens": 2,
            "output_tokens": 3,
            "model": "claude:opus",
            "cost_usd": null
        }),
        None,
    )
    .await
    .unwrap();

    let agent = dur.agents().get("agent-a").await.unwrap();
    assert_eq!(agent.metadata["usage"]["input_tokens"], 7);
    assert_eq!(agent.metadata["usage"]["output_tokens"], 10);
    assert_eq!(agent.metadata["usage"]["cost_usd"], 0.01);
}

#[tokio::test]
async fn register_accepts_extended_kinds_for_future_hosts() {
    let (dur, _dir) = fresh().await;
    for kind in [
        "claude-sdk",
        "gpt-4o",
        "gemini-pro",
        "claude.code",
        "rust_runner",
    ] {
        let mut a = new_agent(&format!("a-{kind}"));
        a.kind = kind.into();
        dur.register_agent(a)
            .await
            .unwrap_or_else(|e| panic!("kind '{kind}' should be accepted: {e}"));
    }
}

#[tokio::test]
async fn register_rejects_empty_kind() {
    let (dur, _dir) = fresh().await;
    let mut a = new_agent("a-empty");
    a.kind = "".into();
    assert!(dur.register_agent(a).await.is_err());
}

#[tokio::test]
async fn register_rejects_uppercase_kind() {
    let (dur, _dir) = fresh().await;
    let mut a = new_agent("a-upper");
    a.kind = "Claude".into();
    assert!(dur.register_agent(a).await.is_err());
}

#[tokio::test]
async fn register_rejects_kind_with_special_chars() {
    let (dur, _dir) = fresh().await;
    for bad in ["shell;rm -rf", "kind with space", "kind/path", "kind@host"] {
        let mut a = new_agent(&format!("a-bad-{}", bad.replace(' ', "_")));
        a.kind = bad.into();
        assert!(
            dur.register_agent(a).await.is_err(),
            "kind '{bad}' must be refused"
        );
    }
}

#[tokio::test]
async fn register_rejects_kind_longer_than_64_chars() {
    let (dur, _dir) = fresh().await;
    let mut a = new_agent("a-long");
    a.kind = "a".repeat(65);
    assert!(dur.register_agent(a).await.is_err());
}
