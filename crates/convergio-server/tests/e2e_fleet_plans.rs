//! ADR-0038 F3-2: end-to-end coverage for `cvg fleet plan` HTTP routes.
//!
//! Boots the daemon in-process and walks the full lifecycle:
//! create → list → show → link a repo (via a per-repo plan created in
//! durability) → add-task. The add-task path proves the cross-crate
//! handoff: the daemon resolves the fleet-plan link to the per-repo
//! plan id, then creates the task through the durability facade.

use convergio_durability::{Durability, NewPlan};
use reqwest::Client;
use serde_json::{json, Value};
use sqlx::Executor as _;

mod common;

#[tokio::test]
async fn fleet_plan_lifecycle_roundtrip() {
    let (base, pool, _dir) = common::boot().await;
    let http = Client::new();

    // --- create ---
    let create: Value = http
        .post(format!("{base}/v1/fleet/plans"))
        .json(&json!({ "title": "cross-repo refactor", "scope": "fleet" }))
        .send()
        .await
        .expect("POST /v1/fleet/plans")
        .json()
        .await
        .expect("decode created");
    let fleet_plan_id = create
        .get("id")
        .and_then(|v| v.as_str())
        .expect("plan.id")
        .to_string();
    assert_eq!(
        create.get("title").and_then(|v| v.as_str()),
        Some("cross-repo refactor")
    );
    assert_eq!(create.get("scope").and_then(|v| v.as_str()), Some("fleet"));

    // --- list (no filter) ---
    let listed: Vec<Value> = http
        .get(format!("{base}/v1/fleet/plans"))
        .send()
        .await
        .expect("GET /v1/fleet/plans")
        .json()
        .await
        .expect("decode list");
    assert_eq!(listed.len(), 1);
    assert_eq!(
        listed[0].get("id").and_then(|v| v.as_str()),
        Some(fleet_plan_id.as_str())
    );

    // --- list (scope filter) ---
    let filtered: Vec<Value> = http
        .get(format!("{base}/v1/fleet/plans?scope=convergio-edu"))
        .send()
        .await
        .expect("GET /v1/fleet/plans?scope=...")
        .json()
        .await
        .expect("decode filtered");
    assert!(
        filtered.is_empty(),
        "scope filter must exclude unrelated plans"
    );

    // --- seed a repo + per-repo plan so the link target exists ---
    pool.inner()
        .execute(
            "INSERT INTO fleet_repos (name, path, language, parser, role) \
             VALUES ('convergio', '/r/convergio', 'rust', 'syn', 'engine')",
        )
        .await
        .expect("seed fleet_repos");
    let durability = Durability::new(pool.clone());
    let repo_plan = durability
        .create_plan(NewPlan {
            title: "convergio: cross-repo refactor".into(),
            project: Some("convergio".into()),
            description: None,
        })
        .await
        .expect("repo plan");

    // --- link a repo (idempotent: call twice) ---
    let link_url = format!("{base}/v1/fleet/plans/{fleet_plan_id}/repos");
    let link_body = json!({ "repo": "convergio", "repo_plan_id": repo_plan.id });
    let link1 = http
        .post(&link_url)
        .json(&link_body)
        .send()
        .await
        .expect("POST link");
    assert!(link1.status().is_success(), "first link must succeed");
    let link2 = http
        .post(&link_url)
        .json(&link_body)
        .send()
        .await
        .expect("POST link (idempotent)");
    assert!(
        link2.status().is_success(),
        "second link must be idempotent"
    );

    // --- show: plan + one link ---
    let view: Value = http
        .get(format!("{base}/v1/fleet/plans/{fleet_plan_id}"))
        .send()
        .await
        .expect("GET show")
        .json()
        .await
        .expect("decode view");
    let links = view
        .get("links")
        .and_then(|v| v.as_array())
        .expect("links array");
    assert_eq!(links.len(), 1, "expected one link after dedup");
    assert_eq!(
        links[0].get("repo").and_then(|v| v.as_str()),
        Some("convergio")
    );

    // --- add task on linked repo ---
    let task_url = format!("{base}/v1/fleet/plans/{fleet_plan_id}/repos/convergio/tasks");
    let task_resp: Value = http
        .post(&task_url)
        .json(&json!({
            "title": "rename FleetPlan accessor",
            "description": "rust-side ripple",
            "wave": 1,
            "sequence": 1,
            "evidence_required": ["code"]
        }))
        .send()
        .await
        .expect("POST task")
        .json()
        .await
        .expect("decode task");
    assert_eq!(
        task_resp.get("repo").and_then(|v| v.as_str()),
        Some("convergio")
    );
    assert_eq!(
        task_resp.get("repo_plan_id").and_then(|v| v.as_str()),
        Some(repo_plan.id.as_str())
    );
    let task = task_resp.get("task").expect("task field");
    assert_eq!(
        task.get("plan_id").and_then(|v| v.as_str()),
        Some(repo_plan.id.as_str()),
        "task must be on the linked per-repo plan"
    );
    assert_eq!(
        task.get("title").and_then(|v| v.as_str()),
        Some("rename FleetPlan accessor")
    );

    // --- error: unknown repo on add-task → 404 ---
    let bad = http
        .post(format!(
            "{base}/v1/fleet/plans/{fleet_plan_id}/repos/no-such-repo/tasks"
        ))
        .json(&json!({"title": "x", "wave": 1, "sequence": 1}))
        .send()
        .await
        .expect("POST bad task");
    assert_eq!(bad.status().as_u16(), 404);

    // --- error: link_repo with an unknown repo_plan_id → 404 ---
    // The link table has no FK to plans, so the route validates at
    // request time to refuse dangling links up front.
    let dangling = http
        .post(format!("{base}/v1/fleet/plans/{fleet_plan_id}/repos"))
        .json(&json!({ "repo": "convergio", "repo_plan_id": "does-not-exist" }))
        .send()
        .await
        .expect("POST dangling link");
    assert_eq!(dangling.status().as_u16(), 404);

    // --- error: link_repo re-targeting same repo to a different
    //     repo_plan_id must 400 (not silently dropped). Create a
    //     second per-repo plan and try to overwrite the existing
    //     link. The store keeps the original target. ---
    let other_repo_plan = durability
        .create_plan(NewPlan {
            title: "convergio: alternate".into(),
            project: Some("convergio".into()),
            description: None,
        })
        .await
        .expect("alternate repo plan");
    let mismatched = http
        .post(format!("{base}/v1/fleet/plans/{fleet_plan_id}/repos"))
        .json(&json!({ "repo": "convergio", "repo_plan_id": other_repo_plan.id }))
        .send()
        .await
        .expect("POST mismatched link");
    assert_eq!(mismatched.status().as_u16(), 400);
    let view: Value = http
        .get(format!("{base}/v1/fleet/plans/{fleet_plan_id}"))
        .send()
        .await
        .expect("GET show after refused relink")
        .json()
        .await
        .expect("decode view");
    let links_after = view.get("links").and_then(|v| v.as_array()).expect("links");
    assert_eq!(links_after.len(), 1);
    assert_eq!(
        links_after[0].get("repo_plan_id").and_then(|v| v.as_str()),
        Some(repo_plan.id.as_str()),
        "refused relink must leave the original target intact"
    );
}
