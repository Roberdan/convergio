//! E2E for `/v1/ontology/*` — T5 of the Ontology Runtime W1
//! plan (ADR-0053).
//!
//! Seeds the daemon's ontology store directly and asserts that the
//! HTTP surface mirrors the crate-level golden export bytes.

mod common;

use common::{boot, client};
use convergio_ontology::{OwnerKind, Store};
use serde_json::{json, Value};

async fn seed_person_shacl(store: &Store) {
    store
        .upsert_object(
            "Person",
            1,
            false,
            "Person",
            "A natural person.",
            json!({}),
            None,
        )
        .await
        .unwrap();
    store
        .upsert_property(
            "email",
            1,
            false,
            "Email",
            "Primary email address.",
            OwnerKind::Object,
            "Person",
            "string",
            true,
            json!({}),
            None,
        )
        .await
        .unwrap();
    store
        .upsert_property(
            "homepage",
            1,
            false,
            "",
            "",
            OwnerKind::Object,
            "Person",
            "iri",
            false,
            json!({}),
            None,
        )
        .await
        .unwrap();
}

async fn seed_person_jsonschema(store: &Store) {
    store
        .upsert_object(
            "Person",
            1,
            false,
            "Person",
            "A natural person.",
            json!({}),
            None,
        )
        .await
        .unwrap();
    store
        .upsert_property(
            "email",
            1,
            false,
            "Email",
            "Primary email address.",
            OwnerKind::Object,
            "Person",
            "string",
            true,
            json!({"maxLength": 254}),
            None,
        )
        .await
        .unwrap();
    store
        .upsert_property(
            "age",
            1,
            false,
            "",
            "",
            OwnerKind::Object,
            "Person",
            "integer",
            false,
            json!({"minimum": 0}),
            None,
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn list_types_returns_seeded_object_with_hash() {
    let (base, pool, _dir) = boot().await;
    seed_person_shacl(&Store::new(pool)).await;
    let body: Value = client()
        .get(format!("{base}/v1/ontology/types"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let objects = body["objects"].as_array().unwrap();
    assert_eq!(objects.len(), 1);
    assert_eq!(objects[0]["name"], "Person");
    assert_eq!(objects[0]["schema_version"], 1);
    assert_eq!(objects[0]["content_hash"].as_str().unwrap().len(), 64);
}

#[tokio::test]
async fn describe_object_inlines_properties() {
    let (base, pool, _dir) = boot().await;
    seed_person_shacl(&Store::new(pool)).await;
    let body: Value = client()
        .get(format!("{base}/v1/ontology/types/object/Person"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body["name"], "Person");
    let props = body["properties"].as_array().unwrap();
    assert_eq!(props.len(), 2);
    assert_eq!(props[0]["name"], "email");
    assert_eq!(props[0]["required"], true);
    assert_eq!(props[1]["name"], "homepage");
}

#[tokio::test]
async fn export_jsonschema_matches_crate_golden() {
    let (base, pool, _dir) = boot().await;
    seed_person_jsonschema(&Store::new(pool)).await;
    let bytes = client()
        .get(format!(
            "{base}/v1/ontology/export/jsonschema/object/Person"
        ))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let actual = std::str::from_utf8(&bytes).unwrap();
    let golden = include_str!("../../convergio-ontology/tests/golden/person_v1.jsonschema.json");
    assert_eq!(
        actual, golden,
        "HTTP JSON-Schema export drifted from crate golden"
    );
}

#[tokio::test]
async fn export_shacl_matches_crate_golden() {
    let (base, pool, _dir) = boot().await;
    seed_person_shacl(&Store::new(pool)).await;
    let bytes = client()
        .get(format!("{base}/v1/ontology/export/shacl/object/Person"))
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let actual = std::str::from_utf8(&bytes).unwrap();
    let golden = include_str!("../../convergio-ontology/tests/golden/person_v1.shacl.jsonld");
    assert_eq!(
        actual, golden,
        "HTTP SHACL export drifted from crate golden"
    );
}

#[tokio::test]
async fn export_unknown_format_is_400() {
    let (base, pool, _dir) = boot().await;
    seed_person_shacl(&Store::new(pool)).await;
    let resp = client()
        .get(format!("{base}/v1/ontology/export/yaml/object/Person"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "ontology_unknown_format");
}

#[tokio::test]
async fn describe_unknown_object_is_404() {
    let (base, _pool, _dir) = boot().await;
    let resp = client()
        .get(format!("{base}/v1/ontology/types/object/Nope"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);
}
