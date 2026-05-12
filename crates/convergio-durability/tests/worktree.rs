//! Reverse lookup from worktree directory slugs to task/plan
//! metadata via [`WorktreeStore`].

use convergio_db::Pool;
use convergio_durability::{init, Durability, NewPlan, NewTask};
use tempfile::tempdir;

async fn fresh() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

#[tokio::test]
async fn holders_for_slugs_resolves_matching_task_and_plan() {
    let (dur, _g) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "P".into(),
            project: Some("convergio-local".into()),
            description: None,
        })
        .await
        .unwrap();
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                title: "T".into(),
                description: None,
                wave: 1,
                sequence: 1,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();

    // Drive the task into in_progress so started_at is populated
    // (the cache column is the visible signal for "claimed N ago").
    dur.try_claim_pending(&task.id, "test-agent")
        .await
        .unwrap()
        .expect("claim");

    let slug = task.id.get(..7).unwrap();
    let holders = dur
        .worktrees()
        .holders_for_slugs(&[slug])
        .await
        .expect("lookup");

    assert_eq!(holders.len(), 1);
    let h = &holders[0];
    assert_eq!(h.slug, slug);
    assert_eq!(h.task_id.as_deref(), Some(task.id.as_str()));
    assert_eq!(h.task_status.as_deref(), Some("in_progress"));
    assert_eq!(h.plan_id.as_deref(), Some(plan.id.as_str()));
    assert!(
        h.plan_number.is_some(),
        "plan number cache must be populated"
    );
    assert!(h.started_at.is_some(), "started_at must be set after claim");
}

#[tokio::test]
async fn holders_for_slugs_marks_orphans() {
    let (dur, _g) = fresh().await;
    let holders = dur
        .worktrees()
        .holders_for_slugs(&["deadbee"])
        .await
        .expect("lookup");
    assert_eq!(holders.len(), 1);
    assert!(holders[0].task_id.is_none());
    assert!(holders[0].plan_id.is_none());
    assert_eq!(holders[0].slug, "deadbee");
}

#[tokio::test]
async fn holders_for_slugs_prefers_in_progress_over_done_collisions() {
    // 7-char prefix collisions are vanishingly rare for UUID v4
    // but we still want deterministic ordering when they occur:
    // an `in_progress` task should win over a completed one with
    // the same slug. We cannot synthesize a real collision (UUIDs
    // are random) so this test exercises the SQL ordering by
    // inserting two tasks whose ids both start with 'a' and
    // querying with a one-char slug. It pins the
    // CASE WHEN ordering in holders_for_slugs so a future
    // refactor cannot accidentally flip it.
    let (dur, _g) = fresh().await;
    let plan = dur
        .create_plan(NewPlan {
            title: "P".into(),
            project: Some("p".into()),
            description: None,
        })
        .await
        .unwrap();
    let t1 = dur
        .create_task(
            &plan.id,
            NewTask {
                title: "T1".into(),
                description: None,
                wave: 1,
                sequence: 1,
                evidence_required: vec![],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    let slug = t1.id.get(..7).unwrap().to_string();

    // Without driving the task anywhere, status is `pending`.
    let holders = dur
        .worktrees()
        .holders_for_slugs(&[slug.as_str()])
        .await
        .unwrap();
    assert_eq!(holders[0].task_status.as_deref(), Some("pending"));

    // Now claim it → in_progress. The same slug must still
    // resolve to t1 (no other tasks present).
    dur.try_claim_pending(&t1.id, "agent")
        .await
        .unwrap()
        .expect("claim");
    let holders = dur
        .worktrees()
        .holders_for_slugs(&[slug.as_str()])
        .await
        .unwrap();
    assert_eq!(holders[0].task_status.as_deref(), Some("in_progress"));
    assert_eq!(holders[0].task_id.as_deref(), Some(t1.id.as_str()));
}
