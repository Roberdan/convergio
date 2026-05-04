//! E2E coverage for the P0.4 `cvg agent` enrichment surfaces:
//! `/v1/agent-registry/agents/summaries`,
//! `/v1/agent-registry/agents/:id/details`, and
//! `/v1/agent-registry/agents/retire-stale`.

mod common;

use common::boot;
use convergio_db::Pool;
use serde_json::{json, Value};

async fn register(client: &reqwest::Client, base: &str, id: &str, kind: &str) {
    client
        .post(format!("{base}/v1/agent-registry/agents"))
        .json(&json!({"id": id, "kind": kind, "host": "test"}))
        .send()
        .await
        .unwrap();
}

async fn set_heartbeat_age(pool: &Pool, agent_id: &str, age_secs: i64) {
    let ts = (chrono::Utc::now() - chrono::Duration::seconds(age_secs)).to_rfc3339();
    sqlx::query("UPDATE agents SET last_heartbeat_at = ?, status = 'idle' WHERE id = ?")
        .bind(&ts)
        .bind(agent_id)
        .execute(pool.inner())
        .await
        .unwrap();
}

#[tokio::test]
async fn summaries_resolves_current_task_title() {
    let (base, _pool, _dir) = boot().await;
    let client = reqwest::Client::new();

    let plan: Value = client
        .post(format!("{base}/v1/plans"))
        .json(&json!({"title": "P0.4 dogfood plan"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let plan_id = plan["id"].as_str().unwrap().to_string();

    let task: Value = client
        .post(format!("{base}/v1/plans/{plan_id}/tasks"))
        .json(&json!({
            "title": "wire enrichment to CLI",
            "wave": 1,
            "sequence": 1,
            "evidence_required": ["test_run"],
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let task_id = task["id"].as_str().unwrap().to_string();

    register(&client, &base, "agent-fresh", "shell").await;
    client
        .post(format!(
            "{base}/v1/agent-registry/agents/agent-fresh/heartbeat"
        ))
        .json(&json!({"current_task_id": task_id}))
        .send()
        .await
        .unwrap();

    let summaries: Vec<Value> = client
        .get(format!("{base}/v1/agent-registry/agents/summaries"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    let row = summaries.iter().find(|s| s["id"] == "agent-fresh").unwrap();
    assert_eq!(row["current_task_title"], "wire enrichment to CLI");
    assert_eq!(row["current_task_status"], "pending");
}

#[tokio::test]
async fn details_includes_all_sections_even_when_empty() {
    let (base, _pool, _dir) = boot().await;
    let client = reqwest::Client::new();
    register(&client, &base, "agent-bare", "shell").await;

    let details: Value = client
        .get(format!(
            "{base}/v1/agent-registry/agents/agent-bare/details"
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(details["id"], "agent-bare");
    assert!(details["leases"].is_array());
    assert_eq!(details["leases"].as_array().unwrap().len(), 0);
    assert!(details["recent_audit"].is_array());
    // The registration emitted `agent.registered` + `agent.session_started`.
    assert!(!details["recent_audit"].as_array().unwrap().is_empty());
    assert!(details["recent_prs"].is_array());
    assert_eq!(details["recent_prs"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn retire_stale_dry_run_then_apply() {
    let (base, pool, _dir) = boot().await;
    let client = reqwest::Client::new();
    register(&client, &base, "agent-fresh", "shell").await;
    register(&client, &base, "agent-stale-1", "shell").await;
    register(&client, &base, "agent-stale-2", "shell").await;

    // 5s old → stays. 45 minutes → should retire under 30-min threshold.
    set_heartbeat_age(&pool, "agent-fresh", 5).await;
    set_heartbeat_age(&pool, "agent-stale-1", 60 * 45).await;
    set_heartbeat_age(&pool, "agent-stale-2", 60 * 60 * 24 * 2).await;

    // Dry run.
    let dry: Value = client
        .post(format!("{base}/v1/agent-registry/agents/retire-stale"))
        .json(&json!({"threshold_seconds": 30 * 60, "apply": false}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(dry["applied"], false);
    let agents = dry["agents"].as_array().unwrap();
    let ids: Vec<&str> = agents
        .iter()
        .map(|a| a["agent_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&"agent-stale-1"));
    assert!(ids.contains(&"agent-stale-2"));
    assert!(!ids.contains(&"agent-fresh"));
    for a in agents {
        assert_eq!(a["retired"], false);
    }

    // Apply.
    let applied: Value = client
        .post(format!("{base}/v1/agent-registry/agents/retire-stale"))
        .json(&json!({"threshold_seconds": 30 * 60, "apply": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(applied["applied"], true);
    for a in applied["agents"].as_array().unwrap() {
        assert_eq!(a["retired"], true);
    }

    // Confirm the rows are now terminated and audit chain still verifies.
    let stale: Value = client
        .get(format!("{base}/v1/agent-registry/agents/agent-stale-1"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(stale["status"], "terminated");

    let verify: Value = client
        .get(format!("{base}/v1/audit/verify"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(verify["ok"], true);
}
