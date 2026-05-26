//! E2E coverage for `?as_of=` + `?tx_as_of=` read query params.

mod common;

use common::{boot, client};
use serde_json::Value;

#[tokio::test]
async fn plans_list_rejects_invalid_as_of() {
    let (base, _pool, _dir) = boot().await;
    let resp = client()
        .get(format!("{base}/v1/plans"))
        .query(&[("as_of", "not-a-timestamp")])
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "invalid_timestamp");
}

#[tokio::test]
async fn fleet_repos_list_rejects_invalid_tx_as_of() {
    let (base, _pool, _dir) = boot().await;
    let resp = client()
        .get(format!("{base}/v1/fleet/repos"))
        .query(&[("tx_as_of", "nope")])
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 422);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "invalid_timestamp");
}
