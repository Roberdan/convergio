//! Graph endpoint E2E tests.
//!
//! Boots the HTTP router in-process, creates a real durability task via
//! the public API, builds a tiny local workspace graph, then asks the
//! server for the task-scoped context pack.

use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::{init, Durability};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tempfile::{Builder, TempDir};
use tokio::net::TcpListener;

async fn boot() -> (String, TempDir) {
    let dir = local_temp_dir("graph-state");
    let db_path = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}?mode=rwc", db_path.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    convergio_ops::init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    convergio_embed::init(&pool).await.unwrap();
    let graph = Arc::new(convergio_graph::Store::new(pool.clone()));
    graph.migrate().await.unwrap();

    let state = AppState {
        durability: Arc::new(Durability::new(pool.clone())),
        ops: Arc::new(convergio_ops::Ops::new(pool.clone())),
        bus: Arc::new(Bus::new(pool.clone())),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph,
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
        fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    (format!("http://{addr}"), dir)
}

fn local_temp_dir(prefix: &str) -> TempDir {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("e2e-workdirs");
    std::fs::create_dir_all(&base).unwrap();
    Builder::new().prefix(prefix).tempdir_in(base).unwrap()
}

fn write_fixture_workspace() -> TempDir {
    let dir = local_temp_dir("graph-workspace");
    let root = dir.path();
    let crate_dir = root.join("crates/graph-e2e-fixture");
    std::fs::create_dir_all(crate_dir.join("src")).unwrap();
    std::fs::create_dir_all(root.join("docs/adr")).unwrap();

    std::fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/graph-e2e-fixture"]
resolver = "2"
"#,
    )
    .unwrap();
    std::fs::write(
        crate_dir.join("Cargo.toml"),
        r#"[package]
name = "graph-e2e-fixture"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    std::fs::write(
        crate_dir.join("src/lib.rs"),
        r#"pub struct WidgetContext {
    pub label: &'static str,
}

pub fn widget_context() -> WidgetContext {
    WidgetContext { label: "local" }
}
"#,
    )
    .unwrap();
    std::fs::write(
        root.join("docs/adr/0099-widget-context.md"),
        r#"---
id: 0099
status: accepted
touches_crates: [graph-e2e-fixture]
---

# Widget context

The fixture crate owns the task context code.
"#,
    )
    .unwrap();
    dir
}

#[tokio::test]
async fn graph_for_task_uses_real_durability_task() {
    let (base, _state_dir) = boot().await;
    let workspace = write_fixture_workspace();
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "graph e2e plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({
            "title": "Wire WidgetContext retrieval",
            "description": "Return graph context for WidgetContext.",
            "evidence_required": []
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    let build: Value = client
        .post(format!("{base}/v1/graph/build"))
        .json(&json!({"manifest_dir": workspace.path(), "force": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(build["crates"], 1);
    assert_eq!(build["files_parsed"], 2);

    let pack: Value = client
        .get(format!(
            "{base}/v1/graph/for-task/{task_id}?node_limit=10&crate=graph-e2e-fixture&adr_required=0099&docs_required=docs/adr/0099-widget-context.md&validation_profile=graph"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(pack["task_id"], task_id);
    assert!(contains_string(&pack["query_tokens"], "widgetcontext"));
    assert!(contains_node(&pack["matched_nodes"], "WidgetContext"));
    assert!(contains_file(
        &pack["files"],
        PathBuf::from("crates/graph-e2e-fixture/src/lib.rs")
    ));
    assert!(contains_adr(&pack["related_adrs"], "0099"));
    assert_eq!(
        pack["structured_metadata"]["crate"],
        json!("graph-e2e-fixture")
    );
    assert_eq!(pack["structured_metadata"]["adr_required"], json!(["0099"]));
    assert_eq!(
        pack["structured_metadata"]["docs_required"],
        json!(["docs/adr/0099-widget-context.md"])
    );
    assert_eq!(
        pack["structured_metadata"]["validation_profile"],
        json!("graph")
    );

    // Without ?semantic, the response carries no `hybrid` block —
    // F1-α / F1-γ contract preserved.
    assert!(pack.get("hybrid").is_none());

    // Seed a couple of embed rows so RRF has both inputs to fuse,
    // then re-query with ?semantic=1 and verify the hybrid section.
    let _: Value = client
        .post(format!("{base}/v1/embed/build"))
        .json(&json!({
            "repo": "convergio",
            "root": workspace.path().to_string_lossy(),
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let hybrid_pack: Value = client
        .get(format!(
            "{base}/v1/graph/for-task/{task_id}?node_limit=10&crate=graph-e2e-fixture&semantic=1&semantic_top_k=5"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let hybrid = &hybrid_pack["hybrid"];
    assert_eq!(hybrid["fusion"], "rrf");
    assert_eq!(hybrid["k"], 60.0);
    assert!(hybrid["hits"].as_array().is_some_and(|h| !h.is_empty()));
    let sources: Vec<&str> = hybrid["hits"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|h| h["match_source"].as_str())
        .collect();
    assert!(
        sources
            .iter()
            .any(|s| matches!(*s, "structural" | "semantic" | "both")),
        "hits must carry match_source provenance: {sources:?}"
    );
}

fn contains_string(values: &Value, needle: &str) -> bool {
    values
        .as_array()
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(needle)))
}

fn contains_node(nodes: &Value, name: &str) -> bool {
    nodes.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["kind"] == "item" && item["name"] == name)
    })
}

fn contains_file(files: &Value, path: PathBuf) -> bool {
    let expected = path.to_string_lossy();
    files.as_array().is_some_and(|items| {
        items
            .iter()
            .any(|item| item["path"].as_str() == Some(expected.as_ref()))
    })
}

fn contains_adr(adrs: &Value, id: &str) -> bool {
    adrs.as_array()
        .is_some_and(|items| items.iter().any(|item| item["adr_id"] == id))
}
