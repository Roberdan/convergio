//! End-to-end tests for `convergio-ontology` storage.
//!
//! Boots a tempdir SQLite via `convergio-db::Pool`, runs the durability
//! migrations (for `plans`), then runs ontology migrations and exercises
//! the store API.

use convergio_db::Pool;
use convergio_ontology::{init, LinkOp, OntologyStore, PropertyOp};
use sqlx::Row;
use tempfile::tempdir;

async fn boot() -> (Pool, String, tempfile::TempDir) {
    let dir = tempdir().expect("tempdir");
    let url = format!(
        "sqlite://{}?mode=rwc",
        dir.path().join("state.db").display()
    );
    let pool = Pool::connect(&url).await.expect("connect");

    // Tenant FK references `plans(id)`.
    convergio_durability::init(&pool)
        .await
        .expect("durability migrations");

    // Create one plan row to serve as `tenant_id`.
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

#[tokio::test]
async fn init_is_idempotent() {
    let (pool, _tenant, _dir) = boot().await;
    init(&pool).await.expect("second init");
}

#[tokio::test]
async fn schema_creates_required_indexes_and_triggers() {
    let (pool, _tenant, _dir) = boot().await;

    let idx: Vec<String> = sqlx::query("SELECT name FROM sqlite_master WHERE type='index'")
        .fetch_all(pool.inner())
        .await
        .expect("index list")
        .into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    assert!(idx.contains(&"idx_object_instances_tenant_type".to_owned()));
    assert!(idx.contains(&"idx_object_links_from_type".to_owned()));
    assert!(idx.contains(&"idx_object_links_to_type".to_owned()));

    let triggers: Vec<String> = sqlx::query("SELECT name FROM sqlite_master WHERE type='trigger'")
        .fetch_all(pool.inner())
        .await
        .expect("trigger list")
        .into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();

    assert!(triggers.contains(&"trg_object_links_no_update".to_owned()));
    assert!(triggers.contains(&"trg_object_links_no_delete".to_owned()));
}

#[tokio::test]
async fn object_links_are_append_only() {
    let (pool, tenant_id, _dir) = boot().await;
    let store = OntologyStore::new(pool.clone());

    let a = store
        .create_instance(&tenant_id, "Person")
        .await
        .expect("create a");
    let b = store
        .create_instance(&tenant_id, "Person")
        .await
        .expect("create b");

    let e = store
        .append_link(&tenant_id, &a.id, &b.id, "knows", LinkOp::Add)
        .await
        .expect("append link");

    let del = sqlx::query("DELETE FROM object_links WHERE id = ?")
        .bind(&e.id)
        .execute(pool.inner())
        .await;
    assert!(del.is_err(), "delete should be refused by trigger");

    let upd = sqlx::query("UPDATE object_links SET op='remove' WHERE id = ?")
        .bind(&e.id)
        .execute(pool.inner())
        .await;
    assert!(upd.is_err(), "update should be refused by trigger");
}

#[tokio::test]
async fn store_enforces_tenant_isolation_on_links_and_properties() {
    let (pool, tenant_a, _dir) = boot().await;

    // Create a second tenant plan.
    let tenant_b = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO plans (id, number, title, description, status, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&tenant_b)
    .bind(2_i64)
    .bind("t2")
    .bind(Option::<String>::None)
    .bind("draft")
    .bind(&now)
    .bind(&now)
    .execute(pool.inner())
    .await
    .expect("insert plan b");

    let store = OntologyStore::new(pool);
    let a = store.create_instance(&tenant_a, "A").await.unwrap();
    let b = store.create_instance(&tenant_b, "B").await.unwrap();

    // Cross-tenant link should fail.
    let bad = store
        .append_link(&tenant_a, &a.id, &b.id, "rel", LinkOp::Add)
        .await;
    assert!(bad.is_err());

    // Cross-tenant property should fail.
    let bad_prop = store
        .append_property(&tenant_a, &b.id, "name", "\"x\"", PropertyOp::Set)
        .await;
    assert!(bad_prop.is_err());
}
