//! Purpose-limitation admission on the ontology write path (ADR-0082).
//!
//! A write to an `ObjectType` flagged `requires_purpose` is refused unless
//! an active, registered purpose is supplied. Types without the flag are
//! unaffected (opt-in).

use convergio_db::Pool;
use convergio_ontology::{init, Error, OntologyStore, PurposeStore, Store};
use serde_json::json;

async fn boot() -> (Pool, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let url = format!(
        "sqlite://{}?mode=rwc",
        dir.path().join("state.db").display()
    );
    let pool = Pool::connect(&url).await.expect("connect");
    convergio_durability::init(&pool)
        .await
        .expect("durability migrations");

    let tenant_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO plans (id, number, title, description, status, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&tenant_id)
    .bind(1_i64)
    .bind("t")
    .bind(Option::<String>::None)
    .bind("draft")
    .bind(&now)
    .bind(&now)
    .execute(pool.inner())
    .await
    .expect("insert plan");

    init(&pool).await.expect("ontology migrations");
    (pool, tenant_id, dir)
}

async fn register_type(pool: &Pool, name: &str, requires_purpose: bool) {
    let body = if requires_purpose {
        json!({ "requires_purpose": true })
    } else {
        json!({})
    };
    Store::new(pool.clone())
        .upsert_object(name, 1, false, name, "", body, None)
        .await
        .expect("upsert object type");
}

#[tokio::test]
async fn unflagged_type_ignores_purpose() {
    let (pool, tenant, _dir) = boot().await;
    register_type(&pool, "Note", false).await;
    OntologyStore::new(pool)
        .create_instance(&tenant, "Note", None)
        .await
        .expect("write allowed without purpose");
}

#[tokio::test]
async fn flagged_type_requires_purpose() {
    let (pool, tenant, _dir) = boot().await;
    register_type(&pool, "StudentRecord", true).await;
    let err = OntologyStore::new(pool)
        .create_instance(&tenant, "StudentRecord", None)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::PurposeRequired { .. }));
}

#[tokio::test]
async fn flagged_type_rejects_unregistered_purpose() {
    let (pool, tenant, _dir) = boot().await;
    register_type(&pool, "StudentRecord", true).await;
    let err = OntologyStore::new(pool)
        .create_instance(&tenant, "StudentRecord", Some("ghost"))
        .await
        .unwrap_err();
    assert!(matches!(err, Error::PurposeMismatch { .. }));
}

#[tokio::test]
async fn flagged_type_accepts_registered_purpose() {
    let (pool, tenant, _dir) = boot().await;
    register_type(&pool, "StudentRecord", true).await;
    PurposeStore::new(pool.clone())
        .register("student-records", "", None)
        .await
        .expect("register purpose");
    OntologyStore::new(pool)
        .create_instance(&tenant, "StudentRecord", Some("student-records"))
        .await
        .expect("write allowed with registered purpose");
}
