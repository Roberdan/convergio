//! Purpose-binding enforcement: every request must carry a valid purpose id.

mod common;

use axum::http::StatusCode;
use serde_json::Value;

#[tokio::test]
async fn missing_or_invalid_purpose_id_is_rejected() {
    let (base, _pool, _dir) = common::boot().await;

    let no_purpose = reqwest::Client::new();
    let with_purpose = common::client();

    let probes: &[(&str, &str)] = &[
        ("GET", "/v1/health"),
        ("GET", "/v1/status"),
        ("GET", "/v1/gates/preconditions"),
        ("GET", "/v1/plans"),
        (
            "GET",
            "/v1/plans/00000000-0000-0000-0000-000000000000/topics",
        ),
        ("GET", "/v1/pr-links"),
        ("GET", "/v1/tasks/00000000-0000-0000-0000-000000000000"),
        (
            "POST",
            "/v1/tasks/00000000-0000-0000-0000-000000000000/context",
        ),
        ("GET", "/v1/crdt/conflicts"),
        ("GET", "/v1/system-messages"),
        ("GET", "/v1/agents"),
        ("GET", "/v1/agent-registry/agents"),
        ("GET", "/v1/audit/events"),
        ("GET", "/v1/audit/verify"),
        ("GET", "/v1/audit/refusals/latest"),
        ("GET", "/v1/audit/events/0/compensate"),
        ("GET", "/v1/capabilities"),
        ("POST", "/v1/dispatch"),
        ("GET", "/v1/workspace/merge-queue"),
        ("GET", "/v1/graph/stats"),
        ("GET", "/v1/embed/stats"),
        ("GET", "/v1/telemetry/series"),
        ("GET", "/v1/fleet/repos"),
        ("GET", "/v1/fleet/patterns"),
        ("GET", "/v1/fleet/duplicates"),
        ("GET", "/v1/fleet/rot"),
        ("GET", "/v1/fleet/doc-drift"),
        ("GET", "/v1/fleet/plans"),
        ("POST", "/v1/solve"),
        (
            "POST",
            "/v1/plans/00000000-0000-0000-0000-000000000000/validate",
        ),
        ("POST", "/v1/audit/append"),
        (
            "GET",
            "/v1/tasks/00000000-0000-0000-0000-000000000000/evidence",
        ),
    ];

    for (method, path) in probes {
        let url = format!("{base}{path}");
        let resp = match *method {
            "GET" => no_purpose.get(&url).send().await.unwrap(),
            "POST" => no_purpose
                .post(&url)
                .json(&serde_json::json!({}))
                .send()
                .await
                .unwrap(),
            "PATCH" => no_purpose
                .patch(&url)
                .json(&serde_json::json!({}))
                .send()
                .await
                .unwrap(),
            _ => panic!("unsupported method: {method}"),
        };
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST, "{method} {path}");
        let body: Value = resp.json().await.unwrap();
        assert_eq!(body["error"]["code"], "purpose_id_missing");

        // With a purpose header, the route should exist (even if it returns
        // another 4xx due to missing body/ids).
        let resp = match *method {
            "GET" => with_purpose.get(&url).send().await.unwrap(),
            "POST" => with_purpose
                .post(&url)
                .json(&serde_json::json!({}))
                .send()
                .await
                .unwrap(),
            "PATCH" => with_purpose
                .patch(&url)
                .json(&serde_json::json!({}))
                .send()
                .await
                .unwrap(),
            _ => unreachable!(),
        };
        let status = resp.status();
        if status == StatusCode::BAD_REQUEST {
            let text = resp.text().await.unwrap_or_default();
            if let Ok(body) = serde_json::from_str::<Value>(&text) {
                assert_ne!(
                    body["error"]["code"], "purpose_id_missing",
                    "{method} {path}"
                );
            }
        }
    }

    // Invalid UUID should be rejected with a stable code.
    let resp = no_purpose
        .get(format!("{base}/v1/health"))
        .header(convergio_api::PURPOSE_ID_HEADER, "not-a-uuid")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "purpose_id_invalid");

    // Happy path sanity check.
    let ok = with_purpose
        .get(format!("{base}/v1/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::OK);
}
