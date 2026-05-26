//! W10 — task taxonomy + skeletal eval outcome ledger (ADR-0063).

use convergio_db::Pool;
use convergio_durability::{
    init, EvalOutcomeStore, NewEvalOutcome, NewPlan, NewTask, TaxonomyStore,
};
use tempfile::tempdir;

async fn fresh() -> (Pool, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (pool, dir)
}

#[tokio::test]
async fn taxonomy_lists_the_closed_seven_kinds() {
    let (pool, _dir) = fresh().await;
    let kinds = TaxonomyStore::new(pool).list().await.unwrap();
    assert_eq!(
        kinds,
        vec![
            "generate-test".to_string(),
            "generic".to_string(),
            "plan".to_string(),
            "refactor".to_string(),
            "review-code".to_string(),
            "summarise".to_string(),
            "write-docs".to_string(),
        ]
    );
}

#[tokio::test]
async fn taxonomy_contains_rejects_unknown_kind() {
    let (pool, _dir) = fresh().await;
    let store = TaxonomyStore::new(pool);
    assert!(store.contains("review-code").await.unwrap());
    assert!(!store.contains("not-a-kind").await.unwrap());
}

#[tokio::test]
async fn eval_outcome_round_trip() {
    let (pool, _dir) = fresh().await;
    let dur = convergio_durability::Durability::new(pool.clone());
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

    let store = EvalOutcomeStore::new(pool);
    let row = store
        .record(NewEvalOutcome {
            task_id: task.id.clone(),
            plan_id: plan.id.clone(),
            runner_kind: "copilot:gpt-5.2".into(),
            taxonomy_kind: "review-code".into(),
            passed: true,
            cost_usd: Some(0.0125),
            latency_ms: Some(8421),
        })
        .await
        .unwrap();
    assert!(row.passed);
    assert_eq!(row.taxonomy_kind, "review-code");

    let n = store
        .count_for("copilot:gpt-5.2", "review-code")
        .await
        .unwrap();
    assert_eq!(n, 1);
}
