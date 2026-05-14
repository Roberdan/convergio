//! Regression: `/v1/status?completed_limit=<negative>` must not return
//! all completed plans.
//!
//! Before the fix, `q.completed_limit as usize` on a negative `i64`
//! wrapped to `usize::MAX`, so the "limit completed plans" filter in
//! `routes/status.rs` accepted everything, while the tasks side
//! (which used `.max(0)`) consistently returned zero. This test
//! pins the two sides to the same behaviour.

use convergio_durability::{Durability, NewPlan, PlanStatus};
use serde_json::Value;

mod common;

#[tokio::test]
async fn status_negative_completed_limit_is_clamped() {
    let (base, pool, _dir) = common::boot().await;
    let durability = Durability::new(pool.clone());

    // Seed three completed plans so a "no clamp" bug would surface
    // as a non-empty `recent_completed_plans` array.
    for i in 0..3 {
        let plan = durability
            .create_plan(NewPlan {
                title: format!("done-{i}"),
                description: None,
                project: None,
            })
            .await
            .expect("create plan");
        durability
            .plans()
            .set_status(&plan.id, PlanStatus::Completed)
            .await
            .expect("set completed");
    }

    let client = reqwest::Client::new();
    let body: Value = client
        .get(format!("{base}/v1/status?completed_limit=-1"))
        .send()
        .await
        .expect("send")
        .error_for_status()
        .expect("status 2xx")
        .json()
        .await
        .expect("json");

    let plans = body["recent_completed_plans"]
        .as_array()
        .expect("recent_completed_plans array");
    let tasks = body["recent_completed_tasks"]
        .as_array()
        .expect("recent_completed_tasks array");

    // Tasks side already clamps via `.max(0)`. Plans side must
    // match: negative limit ⇒ zero completed plans returned.
    assert_eq!(
        plans.len(),
        0,
        "negative completed_limit must clamp completed plans to 0, got {} entries",
        plans.len(),
    );
    assert_eq!(tasks.len(), 0);
}
