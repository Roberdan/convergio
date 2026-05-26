//! Migration smoke tests for `convergio-ops`.

use convergio_db::Pool;
use convergio_ops::init;
use tempfile::tempdir;

#[tokio::test]
async fn ops_workflow_engine_migration_applies() {
    let dir = tempdir().unwrap();
    let url = format!("sqlite://{}/state.db", dir.path().display());
    let pool: Pool = Pool::connect(&url).await.unwrap();
    init(&pool).await.unwrap();

    let applied: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 401")
            .fetch_one(pool.inner())
            .await
            .unwrap();
    assert_eq!(applied, 1);

    let names: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('ops_workflows', 'ops_workflow_instances') ORDER BY name",
    )
    .fetch_all(pool.inner())
    .await
    .unwrap();

    assert_eq!(
        names,
        vec![
            "ops_workflow_instances".to_string(),
            "ops_workflows".to_string()
        ]
    );
}
