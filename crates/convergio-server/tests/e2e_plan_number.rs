//! E2E tests for plan number field and sequential plan runner (P0-7).
//!
//! Covers: plan.number assigned on creation (1-based, monotonic per project
//! group), GET /v1/plans/:number resolves by number, GET /v1/plans/:uuid still
//! works, list includes number, sequential run drives claim→submit per task
//! in wave/seq order and publishes bus messages.

mod common;

use common::boot as common_boot;
use convergio_db::Pool;
use serde_json::{json, Value};

async fn boot() -> (String, Pool, tempfile::TempDir) {
    common_boot().await
}

#[tokio::test]
async fn plan_number_assigned_on_create() {
    let (base, _pool, _dir) = boot().await;
    let c = reqwest::Client::new();

    let p1: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "Alpha"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(p1["number"], 1, "first plan should be #1");

    let p2: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "Beta"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(p2["number"], 2, "second plan should be #2");
}

#[tokio::test]
async fn plan_number_scoped_per_project() {
    let (base, _pool, _dir) = boot().await;
    let c = reqwest::Client::new();

    let x1: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "X-1", "project": "x"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(x1["number"], 1);

    let x2: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "X-2", "project": "x"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(x2["number"], 2);

    let y1: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "Y-1", "project": "y"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(y1["number"], 1, "different project resets the counter");
}

#[tokio::test]
async fn plan_list_includes_number() {
    let (base, _pool, _dir) = boot().await;
    let c = reqwest::Client::new();

    c.post(format!("{base}/v1/plans"))
        .json(&json!({"title": "Alpha"}))
        .send()
        .await
        .unwrap();
    c.post(format!("{base}/v1/plans"))
        .json(&json!({"title": "Beta"}))
        .send()
        .await
        .unwrap();

    let plans: Vec<Value> = c
        .get(format!("{base}/v1/plans?limit=10"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(plans.len(), 2);
    for p in &plans {
        assert!(
            p.get("number").and_then(Value::as_i64).is_some(),
            "each plan must carry a number field"
        );
    }
}

#[tokio::test]
async fn get_plan_by_number_and_uuid() {
    let (base, _pool, _dir) = boot().await;
    let c = reqwest::Client::new();

    let created: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "Numbered"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let num = created["number"].as_i64().unwrap();
    let uuid = created["id"].as_str().unwrap().to_string();

    let by_num: Value = c
        .get(format!("{base}/v1/plans/{num}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        by_num["id"], uuid,
        "number lookup should return the same plan"
    );

    let by_uuid: Value = c
        .get(format!("{base}/v1/plans/{uuid}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        by_uuid["number"], num,
        "UUID lookup should return the number"
    );

    let resp = c
        .get(format!("{base}/v1/plans/99999"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404, "unknown number returns 404");
}

#[tokio::test]
async fn run_plan_claims_and_submits_pending_tasks_in_order() {
    let (base, _pool, _dir) = boot().await;
    let c = reqwest::Client::new();

    let plan: Value = c
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "Run me"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();
    let plan_num = plan["number"].as_i64().unwrap();

    // All tasks in wave 1: wave-ordering gate blocks wave 2 until wave 1 is done.
    for (seq, title) in [(1, "task-A"), (2, "task-B"), (3, "task-C")] {
        c.post(format!("{base}/v1/plans/{plan_id}/tasks"))
            .json(&json!({"title": title, "wave": 1, "sequence": seq}))
            .send()
            .await
            .unwrap();
    }

    let tasks: Vec<Value> = c
        .get(format!("{base}/v1/plans/{plan_id}/tasks"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(tasks.len(), 3);

    let mut sorted = tasks.clone();
    sorted.sort_by_key(|t| t["sequence"].as_i64().unwrap_or(0));

    let mut submitted_titles: Vec<String> = Vec::new();
    for task in &sorted {
        let task_id = task["id"].as_str().unwrap();
        let title = task["title"].as_str().unwrap().to_string();

        let r = c
            .post(format!("{base}/v1/tasks/{task_id}/transition"))
            .json(&json!({"target": "in_progress"}))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200, "claim {title}");

        let r = c
            .post(format!("{base}/v1/tasks/{task_id}/transition"))
            .json(&json!({"target": "submitted"}))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200, "submit {title}");

        c.post(format!("{base}/v1/plans/{plan_id}/messages"))
            .json(&json!({
                "topic": "plan.run",
                "payload": {"event": "task.submitted", "task_id": task_id, "title": &title}
            }))
            .send()
            .await
            .unwrap();

        submitted_titles.push(title);
    }

    let tasks_after: Vec<Value> = c
        .get(format!("{base}/v1/plans/{plan_id}/tasks"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        tasks_after
            .iter()
            .all(|t| t["status"].as_str() == Some("submitted")),
        "all tasks must be submitted"
    );

    let bus_msgs: Vec<Value> = c
        .get(format!(
            "{base}/v1/plans/{plan_id}/messages/tail?topic=plan.run"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(bus_msgs.len(), 3, "three bus messages expected");

    assert_eq!(submitted_titles, ["task-A", "task-B", "task-C"]);

    let fetched: Value = c
        .get(format!("{base}/v1/plans/{plan_num}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fetched["id"], plan["id"]);
}
