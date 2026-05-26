//! ADR-0038 F3-4: cross-repo audit-chain verify.

use convergio_durability::{Durability, NewPlan};
use reqwest::Client;
use serde_json::{json, Value};
use sqlx::Executor as _;

mod common;

#[tokio::test]
async fn fleet_audit_verify_detects_tampering() {
    let (base, pool, _d) = common::boot().await;
    let http = Client::new();
    let durability = Durability::new(pool.clone());
    let create: Value = http
        .post(format!("{base}/v1/fleet/plans"))
        .json(&json!({ "title": "audit fleet", "scope": "fleet" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let fid = create["id"].as_str().unwrap().to_string();
    for repo in ["convergio", "convergio-edu"] {
        pool.inner()
            .execute(
                sqlx::query(
                    "INSERT INTO fleet_repos (name, path, language, parser, role) \
                     VALUES (?, ?, 'rust', 'syn', 'engine')",
                )
                .bind(repo)
                .bind(format!("/r/{repo}")),
            )
            .await
            .unwrap();
        let plan = durability
            .create_plan(NewPlan {
                title: format!("{repo}: audit"),
                project: Some(repo.into()),
                description: None,
                no_dispatch_default: false,
            })
            .await
            .unwrap();
        http.post(format!("{base}/v1/fleet/plans/{fid}/repos"))
            .json(&json!({ "repo": repo, "repo_plan_id": plan.id }))
            .send()
            .await
            .unwrap()
            .error_for_status()
            .unwrap();
    }
    let url = format!("{base}/v1/fleet/plans/{fid}/audit-verify");

    // Sanity: clean chain → every verdict pass, aggregate passing.
    let r: Value = http.get(&url).send().await.unwrap().json().await.unwrap();
    assert_eq!(r["passing"].as_bool(), Some(true), "{r}");
    let vs = r["verdicts"].as_array().unwrap();
    assert_eq!(vs.len(), 2);
    for v in vs {
        assert_eq!(v["ok"].as_bool(), Some(true), "{v}");
        assert!(v["checked"].as_i64().unwrap_or(0) > 0, "{v}");
    }

    // Tamper the tail row's prev_hash. Verifier walks in seq order
    // and trips at that seq.
    let (target,): (i64,) = sqlx::query_as("SELECT MAX(seq) FROM audit_log")
        .fetch_one(pool.inner())
        .await
        .unwrap();
    pool.inner()
        .execute(sqlx::query("UPDATE audit_log SET prev_hash='tampered' WHERE seq=?").bind(target))
        .await
        .unwrap();
    let r: Value = http.get(&url).send().await.unwrap().json().await.unwrap();
    assert_eq!(r["passing"].as_bool(), Some(false), "{r}");
    for v in r["verdicts"].as_array().unwrap() {
        assert_eq!(v["ok"].as_bool(), Some(false), "{v}");
        assert_eq!(v["broken_at"].as_i64(), Some(target), "{v}");
    }

    // Codex P2 #377: zero linked repos must not mask the now-tampered
    // chain. Reuse the existing tampering via a second fleet plan.
    let u: Value = http
        .post(format!("{base}/v1/fleet/plans"))
        .json(&json!({ "title": "unlinked", "scope": "fleet" }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let r: Value = http
        .get(format!(
            "{base}/v1/fleet/plans/{}/audit-verify",
            u["id"].as_str().unwrap()
        ))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r["passing"].as_bool(), Some(false), "{r}");
    assert!(r["verdicts"].as_array().unwrap().is_empty());
}
