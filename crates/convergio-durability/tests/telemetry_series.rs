//! Integration tests for `telemetry_series` (migration 0014).
//!
//! Tests drive `record_telemetry_snapshot` and `query_telemetry_series`
//! directly — no wall-clock sleep required.

use convergio_db::Pool;
use convergio_durability::{init, Durability};
use tempfile::tempdir;

async fn fresh_durability() -> (Durability, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();
    (Durability::new(pool), dir)
}

#[tokio::test]
async fn migration_creates_table_and_index() {
    let (dur, _dir) = fresh_durability().await;

    let table: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master \
         WHERE type='table' AND name='telemetry_series'",
    )
    .fetch_one(dur.pool().inner())
    .await
    .unwrap();
    assert_eq!(table, 1, "telemetry_series table must exist");

    let idx: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master \
         WHERE type='index' AND name='idx_telemetry_series_bucket'",
    )
    .fetch_one(dur.pool().inner())
    .await
    .unwrap();
    assert_eq!(idx, 1, "bucket index must exist");

    let applied: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 14")
            .fetch_one(dur.pool().inner())
            .await
            .unwrap();
    assert_eq!(applied, 1, "migration 0014 must be recorded");
}

#[tokio::test]
async fn snapshot_inserts_all_seven_metrics() {
    let (dur, _dir) = fresh_durability().await;
    dur.record_telemetry_snapshot().await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM telemetry_series")
        .fetch_one(dur.pool().inner())
        .await
        .unwrap();
    assert_eq!(count, 7, "one row per metric counter");
}

#[tokio::test]
async fn snapshot_is_idempotent_within_same_minute() {
    let (dur, _dir) = fresh_durability().await;
    dur.record_telemetry_snapshot().await.unwrap();
    dur.record_telemetry_snapshot().await.unwrap();

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM telemetry_series")
        .fetch_one(dur.pool().inner())
        .await
        .unwrap();
    assert_eq!(
        count, 7,
        "upsert must not duplicate rows in the same minute"
    );
}

#[tokio::test]
async fn query_series_returns_recorded_points() {
    let (dur, _dir) = fresh_durability().await;
    dur.record_telemetry_snapshot().await.unwrap();

    let points = dur.query_telemetry_series("plans_active", 1).await.unwrap();
    assert_eq!(points.len(), 1);
    assert_eq!(points[0].metric, "plans_active");
}

#[tokio::test]
async fn query_series_unknown_metric_returns_empty() {
    let (dur, _dir) = fresh_durability().await;
    dur.record_telemetry_snapshot().await.unwrap();

    let points = dur
        .query_telemetry_series("nonexistent_metric", 7)
        .await
        .unwrap();
    assert!(points.is_empty());
}

#[tokio::test]
async fn query_series_caps_window_at_seven_days() {
    let (dur, _dir) = fresh_durability().await;
    dur.record_telemetry_snapshot().await.unwrap();

    // window_days=999 must be capped to 7 and still return data
    let points = dur
        .query_telemetry_series("audit_rows_total", 999)
        .await
        .unwrap();
    assert_eq!(points.len(), 1, "capped window still returns recent data");
}

#[tokio::test]
async fn prune_removes_old_rows() {
    use chrono::Timelike as _;
    use convergio_durability::store::TelemetrySeriesStore;

    let (dur, _dir) = fresh_durability().await;
    let pool = dur.pool().inner();
    let store = TelemetrySeriesStore::new(pool);

    // Insert a row from 8 days ago
    let old_bucket = (chrono::Utc::now() - chrono::Duration::days(8))
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
        .to_rfc3339();
    store.upsert(&old_bucket, "plans_active", 5).await.unwrap();

    // Insert a fresh row for today
    let now_bucket = chrono::Utc::now()
        .with_second(0)
        .unwrap()
        .with_nanosecond(0)
        .unwrap()
        .to_rfc3339();
    store.upsert(&now_bucket, "plans_active", 1).await.unwrap();

    let count_before: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM telemetry_series")
        .fetch_one(pool)
        .await
        .unwrap();
    // 1 old + 1 fresh (different bucket_ts)
    assert_eq!(
        count_before, 2,
        "should have old and fresh row before prune"
    );

    // Prune with cutoff = now - 7 days, which removes the 8-day-old row
    let cutoff = chrono::Utc::now() - chrono::Duration::days(7);
    let deleted = store.prune(&cutoff).await.unwrap();
    assert_eq!(deleted, 1, "exactly one old row must be deleted");

    let count_after: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM telemetry_series")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(count_after, 1, "only fresh row remains after prune");
}
