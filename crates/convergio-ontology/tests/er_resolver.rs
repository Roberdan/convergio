//! End-to-end tests for the deterministic entity-resolution engine
//! (ADR-0055): exact-duplicate grouping, normalization, no false
//! positives on distinct entities, deterministic ordering, and the
//! reversible merge record.

use convergio_db::Pool;
use convergio_ontology::{init, EntityResolver, MatchRule, OntologyStore, PropertyOp, Store};
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
    Store::new(pool.clone())
        .upsert_object("Person", 1, false, "Person", "", json!({}), None)
        .await
        .expect("register Person type");
    (pool, tenant_id, dir)
}

/// Create a `Person` instance and set each `(prop, value)` pair.
async fn person(store: &OntologyStore, tenant: &str, props: &[(&str, &str)]) -> String {
    let inst = store
        .create_instance(tenant, "Person", None)
        .await
        .expect("create instance");
    for (name, value) in props {
        store
            .append_property(
                tenant,
                &inst.id,
                name,
                &json!(value).to_string(),
                PropertyOp::Set,
            )
            .await
            .expect("set property");
    }
    inst.id
}

#[tokio::test]
async fn groups_exact_duplicates() {
    let (pool, tenant, _dir) = boot().await;
    let store = OntologyStore::new(pool.clone());
    let a = person(&store, &tenant, &[("email", "alice@example.com")]).await;
    let b = person(&store, &tenant, &[("email", "alice@example.com")]).await;
    let _c = person(&store, &tenant, &[("email", "bob@example.com")]).await;

    let rule = MatchRule::new("Person", ["email"]);
    let groups = EntityResolver::new(pool)
        .candidates(&tenant, "Person", &rule)
        .await
        .expect("candidates");

    assert_eq!(groups.len(), 1, "only the alice pair is a duplicate");
    let mut expected = vec![a, b];
    expected.sort();
    assert_eq!(groups[0].members, expected);
    assert!(groups[0].explanation.contains("alice@example.com"));
    assert!(groups[0].explanation.contains("email"));
}

#[tokio::test]
async fn normalizes_case_and_whitespace() {
    let (pool, tenant, _dir) = boot().await;
    let store = OntologyStore::new(pool.clone());
    person(&store, &tenant, &[("name", "  Ada   Lovelace ")]).await;
    person(&store, &tenant, &[("name", "ada lovelace")]).await;
    person(&store, &tenant, &[("name", "ADA\tLOVELACE")]).await;

    let rule = MatchRule::new("Person", ["name"]);
    let groups = EntityResolver::new(pool)
        .candidates(&tenant, "Person", &rule)
        .await
        .expect("candidates");

    assert_eq!(groups.len(), 1, "all three normalize to the same key");
    assert_eq!(groups[0].members.len(), 3);
    assert_eq!(
        groups[0].fields,
        vec![("name".to_string(), "ada lovelace".to_string())]
    );
}

#[tokio::test]
async fn distinct_entities_are_not_grouped() {
    let (pool, tenant, _dir) = boot().await;
    let store = OntologyStore::new(pool.clone());
    person(&store, &tenant, &[("email", "alice@example.com")]).await;
    person(&store, &tenant, &[("email", "bob@example.com")]).await;
    person(&store, &tenant, &[("email", "carol@example.com")]).await;

    let rule = MatchRule::new("Person", ["email"]);
    let groups = EntityResolver::new(pool)
        .candidates(&tenant, "Person", &rule)
        .await
        .expect("candidates");

    assert!(
        groups.is_empty(),
        "three distinct emails produce no duplicate candidates"
    );
}

#[tokio::test]
async fn missing_key_property_is_excluded() {
    let (pool, tenant, _dir) = boot().await;
    let store = OntologyStore::new(pool.clone());
    person(
        &store,
        &tenant,
        &[("email", "alice@example.com"), ("name", "Alice")],
    )
    .await;
    person(&store, &tenant, &[("email", "alice@example.com")]).await; // no name

    let rule = MatchRule::new("Person", ["email", "name"]);
    let groups = EntityResolver::new(pool)
        .candidates(&tenant, "Person", &rule)
        .await
        .expect("candidates");

    assert!(
        groups.is_empty(),
        "the instance missing `name` cannot be keyed"
    );
}

#[tokio::test]
async fn latest_property_value_wins() {
    let (pool, tenant, _dir) = boot().await;
    let store = OntologyStore::new(pool.clone());
    let a = store
        .create_instance(&tenant, "Person", None)
        .await
        .unwrap();
    // First set a non-matching email, then update to the duplicate value.
    store
        .append_property(
            &tenant,
            &a.id,
            "email",
            &json!("old@example.com").to_string(),
            PropertyOp::Set,
        )
        .await
        .unwrap();
    store
        .append_property(
            &tenant,
            &a.id,
            "email",
            &json!("new@example.com").to_string(),
            PropertyOp::Set,
        )
        .await
        .unwrap();
    person(&store, &tenant, &[("email", "new@example.com")]).await;

    let rule = MatchRule::new("Person", ["email"]);
    let groups = EntityResolver::new(pool)
        .candidates(&tenant, "Person", &rule)
        .await
        .expect("candidates");

    assert_eq!(groups.len(), 1, "the latest email value drives the match");
    assert_eq!(groups[0].fields[0].1, "new@example.com");
}

#[tokio::test]
async fn unset_property_removes_it_from_state() {
    let (pool, tenant, _dir) = boot().await;
    let store = OntologyStore::new(pool.clone());
    let a = store
        .create_instance(&tenant, "Person", None)
        .await
        .unwrap();
    store
        .append_property(
            &tenant,
            &a.id,
            "email",
            &json!("alice@example.com").to_string(),
            PropertyOp::Set,
        )
        .await
        .unwrap();
    store
        .append_property(
            &tenant,
            &a.id,
            "email",
            &json!("alice@example.com").to_string(),
            PropertyOp::Unset,
        )
        .await
        .unwrap();
    person(&store, &tenant, &[("email", "alice@example.com")]).await;

    let rule = MatchRule::new("Person", ["email"]);
    let groups = EntityResolver::new(pool)
        .candidates(&tenant, "Person", &rule)
        .await
        .expect("candidates");

    assert!(
        groups.is_empty(),
        "an unset email leaves only one keyed instance"
    );
}

#[tokio::test]
async fn group_and_member_ordering_is_deterministic() {
    let (pool, tenant, _dir) = boot().await;
    let store = OntologyStore::new(pool.clone());
    // Two separate duplicate clusters; insert in mixed order.
    person(&store, &tenant, &[("email", "zoe@example.com")]).await;
    person(&store, &tenant, &[("email", "alice@example.com")]).await;
    person(&store, &tenant, &[("email", "zoe@example.com")]).await;
    person(&store, &tenant, &[("email", "alice@example.com")]).await;

    let rule = MatchRule::new("Person", ["email"]);
    let resolver = EntityResolver::new(pool);
    let first = resolver.candidates(&tenant, "Person", &rule).await.unwrap();
    let second = resolver.candidates(&tenant, "Person", &rule).await.unwrap();

    assert_eq!(first, second, "repeated runs are byte-identical");
    assert_eq!(first.len(), 2);
    // Groups ordered by canonical key: alice before zoe.
    assert_eq!(first[0].fields[0].1, "alice@example.com");
    assert_eq!(first[1].fields[0].1, "zoe@example.com");
    for group in &first {
        let mut sorted = group.members.clone();
        sorted.sort();
        assert_eq!(group.members, sorted, "members are sorted");
    }
}

#[tokio::test]
async fn merge_is_recorded_and_reversible() {
    let (pool, tenant, _dir) = boot().await;
    let store = OntologyStore::new(pool.clone());
    let a = person(&store, &tenant, &[("email", "alice@example.com")]).await;
    let b = person(&store, &tenant, &[("email", "alice@example.com")]).await;

    let resolver = EntityResolver::new(pool.clone());
    let merge = resolver
        .record_merge(&tenant, &a, &b)
        .await
        .expect("record merge");
    assert_eq!(merge.link_type, "er:same-as");
    assert_eq!(merge.op, "add");

    let unmerge = resolver
        .record_unmerge(&tenant, &a, &b)
        .await
        .expect("record unmerge");
    assert_eq!(unmerge.op, "remove");

    // Both events persist in the append-only log (auditable history).
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM object_links WHERE link_type = 'er:same-as'")
            .fetch_one(pool.inner())
            .await
            .unwrap();
    assert_eq!(count, 2);
}
