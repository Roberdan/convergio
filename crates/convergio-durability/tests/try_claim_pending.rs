//! Atomic-claim regression for the executor duplicate-dispatch race.
//!
//! Pre-2026-05-12 the executor `dispatch_one` spawned the agent
//! subprocess *before* transitioning the task to `in_progress`. Two
//! concurrent dispatch ticks could both see a `pending` task and both
//! spawn against it. The audit flagged this HIGH (P1 ownership
//! invariant) — `convergio-executor/src/executor.rs:89,108`.
//!
//! `Durability::try_claim_pending` now does a conditional UPDATE in
//! the same transaction as the audit row, so exactly one caller can
//! win the claim. This file pins that property with a 16-way
//! concurrent claim against a single pending task.

use convergio_db::Pool;
use convergio_durability::{init, Durability, NewPlan, NewTask};
use tempfile::tempdir;
use tokio::task::JoinSet;

async fn fresh_dur() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let pool = Pool::connect(&format!("sqlite://{}/state.db", dir.path().display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_claims_exactly_one_winner() {
    let (dur, _dir) = fresh_dur().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
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
                title: "t".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    let mut jobs: JoinSet<Option<String>> = JoinSet::new();
    for i in 0..16 {
        let d = dur.clone();
        let tid = task.id.clone();
        let agent = format!("agent-{i:02}");
        jobs.spawn(async move {
            d.try_claim_pending(&tid, &agent)
                .await
                .unwrap()
                .map(|t| t.agent_id.unwrap_or_default())
        });
    }

    let mut winners = 0usize;
    let mut losers = 0usize;
    let mut winner_agent: Option<String> = None;
    while let Some(res) = jobs.join_next().await {
        match res.unwrap() {
            Some(agent) => {
                winners += 1;
                winner_agent = Some(agent);
            }
            None => losers += 1,
        }
    }
    assert_eq!(winners, 1, "exactly one claim must win, got {winners}");
    assert_eq!(losers, 15, "every loser must see None, got {losers}");

    // Final state: in_progress, owned by the winner.
    let final_task = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(final_task.status.as_str(), "in_progress");
    assert_eq!(final_task.agent_id, winner_agent);

    // Exactly one task.in_progress audit row.
    let n: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM audit_log WHERE entity_id = ? AND transition = 'task.in_progress'",
    )
    .bind(&task.id)
    .fetch_one(dur.pool().inner())
    .await
    .unwrap();
    assert_eq!(
        n.0, 1,
        "exactly one task.in_progress audit row, got {}",
        n.0
    );
}

#[tokio::test]
async fn second_claim_is_none_after_first_wins() {
    let (dur, _dir) = fresh_dur().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "p".into(),
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
                title: "t".into(),
                description: None,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    let first = dur.try_claim_pending(&task.id, "alpha").await.unwrap();
    assert!(first.is_some(), "first claim should win");

    let second = dur.try_claim_pending(&task.id, "beta").await.unwrap();
    assert!(
        second.is_none(),
        "second claim must observe non-pending row"
    );

    let final_task = dur.tasks().get(&task.id).await.unwrap();
    assert_eq!(final_task.agent_id.as_deref(), Some("alpha"));
}
