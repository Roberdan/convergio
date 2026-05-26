//! W10 — task taxonomy + skeletal eval outcome ledger (ADR-0063).

use convergio_db::Pool;
use convergio_durability::{init, EvalOutcomeStore, NewEvalOutcome, TaxonomyStore};
use tempfile::tempdir;

async fn fresh() -> (Pool, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (pool, dir)
}

#[tokio::test]
async fn taxonomy_closed_seven_kinds_and_contains() {
    let (pool, _dir) = fresh().await;
    let store = TaxonomyStore::new(pool);
    let kinds = store.list().await.unwrap();
    assert_eq!(kinds.len(), 7);
    assert!(kinds.contains(&"review-code".to_string()));
    assert!(store.contains("review-code").await.unwrap());
    assert!(!store.contains("not-a-kind").await.unwrap());
}

#[tokio::test]
async fn eval_outcome_round_trip() {
    let (pool, _dir) = fresh().await;
    let store = EvalOutcomeStore::new(pool);
    let row = store
        .record(NewEvalOutcome {
            task_id: "t1".into(),
            plan_id: "p1".into(),
            runner_kind: "copilot:gpt-5.2".into(),
            taxonomy_kind: "review-code".into(),
            passed: true,
            cost_usd: Some(0.0125),
            latency_ms: Some(8421),
        })
        .await
        .unwrap();
    assert!(row.passed);
    let n = store
        .count_for("copilot:gpt-5.2", "review-code")
        .await
        .unwrap();
    assert_eq!(n, 1);
}
