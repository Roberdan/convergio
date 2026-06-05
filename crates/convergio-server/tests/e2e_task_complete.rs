//! E2E test for `cvg task complete` orchestration (P1-1).
//!
//! Verifies the full completion pipeline against an in-process daemon:
//!   graph for-task → embed for-task → evidence add ×3
//!   → transition submitted → validate plan (Thor) → done
//!   → audit chain intact.

mod common;

use serde_json::{json, Value};

/// Replicates the exact HTTP sequence that `cvg task complete --pr` drives.
#[tokio::test]
async fn task_complete_full_flow_produces_done_with_intact_audit() {
    let (base, pool, _dir) = common::boot().await;
    // Embed tables are not migrated by common::boot — init them here so
    // that /v1/embed/for-task and ?semantic=1 work in this test.
    convergio_embed::init(&pool).await.expect("embed init");
    let c = common::client();

    // --- Setup -----------------------------------------------------------
    let plan: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "complete flow plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    let task: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "implement the orchestrator"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    // Claim.
    let _: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "test-agent-p1-1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // --- Step 1: graph for-task --semantic (empty graph is fine) ----------
    // The orchestrator uses best-effort: HTTP 200 or error both accepted.
    // We collect whatever JSON comes back (possibly an error body) to build
    // the evidence summary — same pattern as task_complete.rs.
    let pack: Value = c
        .get(format!(
            "{base}/v1/graph/for-task/{task_id}?semantic=1&semantic_top_k=25"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap_or(json!({"ok": false, "skipped": true}));

    // --- Step 2: embed for-task (empty store returns empty hits) ----------
    let embed: Value = c
        .post(format!("{base}/v1/embed/for-task"))
        .json(&json!({"query": "implement the orchestrator", "top_k": 10}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(embed["ok"], true);

    // --- Step 3: evidence graph_pack -------------------------------------
    let ev1: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "graph_pack",
            "payload": {
                "matched_nodes": pack["matched_nodes"].as_array().map(|a| a.len()).unwrap_or(0),
                "files": pack["files"].as_array().map(|a| a.len()).unwrap_or(0),
                "estimated_tokens": pack.get("estimated_tokens"),
            }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ev1["kind"], "graph_pack");

    // --- Step 4: evidence embed_query ------------------------------------
    let ev2: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "embed_query",
            "payload": {
                "hit_count": embed["hits"].as_array().map(|a| a.len()).unwrap_or(0),
            }
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ev2["kind"], "embed_query");

    // --- Step 5: evidence pr_link ----------------------------------------
    let ev3: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/evidence"))
        .json(&json!({
            "kind": "pr_link",
            "payload": {"pr": 42}
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ev3["kind"], "pr_link");

    // --- Step 6: transition → submitted ----------------------------------
    let submitted: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "submitted", "agent_id": "test-agent-p1-1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        submitted["status"], "submitted",
        "expected submitted: {submitted}"
    );

    // --- Step 7: validate plan (Thor) → done ------------------------------
    let verdict: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verdict["verdict"], "pass", "Thor verdict: {verdict}");

    // --- Verify: task is done --------------------------------------------
    let done: Value = c
        .get(format!("{base}/v1/tasks/{task_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        done["status"], "done",
        "task must be done after Thor: {done}"
    );

    // --- Verify: all 3 evidence rows present -----------------------------
    let evidence: Value = c
        .get(format!("{base}/v1/tasks/{task_id}/evidence"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let kinds: Vec<&str> = evidence
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["kind"].as_str())
        .collect();
    assert!(
        kinds.contains(&"graph_pack"),
        "evidence must include graph_pack: {kinds:?}"
    );
    assert!(
        kinds.contains(&"embed_query"),
        "evidence must include embed_query: {kinds:?}"
    );
    assert!(
        kinds.contains(&"pr_link"),
        "evidence must include pr_link: {kinds:?}"
    );

    // --- Verify: audit chain intact --------------------------------------
    let report: Value = c
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(report["ok"], true, "audit chain must be intact");
}

/// When all tasks in a plan are submitted, a second validate is idempotent.
#[tokio::test]
async fn task_complete_validate_idempotent() {
    let (base, _pool, _dir) = common::boot().await;
    let c = common::client();

    let plan: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "idempotent validate"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap();

    let task: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({"title": "small task"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap();

    let _: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "in_progress", "agent_id": "test-agent"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let _: Value = c
        .post(format!("{base}/v1/tasks/{task_id}/transition"))
        .json(&json!({"target": "submitted", "agent_id": "test-agent"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let v1: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v1["verdict"], "pass");

    let v2: Value = c
        .post(format!("{base}/v1/plans/{plan_id}/validate"))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v2["verdict"], "pass", "re-validate must be idempotent");
}
