//! E2E round-trip tests for the three ontology MCP actions.
//!
//! These mirror the fleet_plan round-trip pattern: spin up a stub axum
//! daemon, dispatch each action through the Bridge, and assert the HTTP
//! verb + path + payload reaching the daemon match what an MCP client
//! would expect (ADR-0047, ADR-0053).
//!
//! Byte-identity for `ontology.export` is also asserted: the helper
//! must surface the daemon's raw response bytes unchanged so JSON-Schema
//! and SHACL exports remain rerun-deterministic (ADR-0060).

use crate::bridge::Bridge;
use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
    Router,
};
use convergio_api::{ActRequest, Action, SCHEMA_VERSION};
use serde_json::json;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::net::TcpListener;

#[derive(Default)]
struct Calls {
    log: StdMutex<Vec<(String, String)>>,
}

#[tokio::test]
async fn ontology_actions_round_trip_to_expected_paths() {
    let calls = Arc::new(Calls::default());
    let state = calls.clone();
    let app = Router::new()
        .route(
            "/v1/ontology/types",
            get(|State(s): State<Arc<Calls>>| async move {
                s.log
                    .lock()
                    .unwrap()
                    .push(("GET".into(), "/v1/ontology/types".into()));
                axum::Json(json!({"types": []}))
            }),
        )
        .route(
            "/v1/ontology/types/:kind/:name",
            get(
                |State(s): State<Arc<Calls>>, Path((kind, name)): Path<(String, String)>| async move {
                    s.log
                        .lock()
                        .unwrap()
                        .push(("GET".into(), format!("/v1/ontology/types/{kind}/{name}")));
                    axum::Json(json!({"type": {"name": name, "kind": kind}}))
                },
            ),
        )
        .route(
            "/v1/ontology/export/:format/object/:name",
            get(
                |State(s): State<Arc<Calls>>,
                 Path((format, name)): Path<(String, String)>,
                 Query(q): Query<HashMap<String, String>>| async move {
                    let suffix = if let Some(v) = q.get("version") {
                        format!("?version={v}")
                    } else {
                        String::new()
                    };
                    s.log.lock().unwrap().push((
                        "GET".into(),
                        format!("/v1/ontology/export/{format}/object/{name}{suffix}"),
                    ));
                    let body = "{\"$schema\":\"http://json-schema.org/draft-07/schema#\"}\n";
                    (
                        StatusCode::OK,
                        [(header::CONTENT_TYPE, "application/json")],
                        body,
                    )
                        .into_response()
                },
            ),
        )
        .with_state(state);
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let bridge = Bridge::new(url);

    let list = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::OntologyList,
            params: json!({}),
        })
        .await;
    assert!(list.ok, "list: {list:?}");

    let describe = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::OntologyDescribe,
            params: json!({"kind": "object", "name": "Person"}),
        })
        .await;
    assert!(describe.ok, "describe: {describe:?}");

    let export = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::OntologyExport,
            params: json!({"name": "Person", "format": "jsonschema"}),
        })
        .await;
    assert!(export.ok, "export: {export:?}");
    let data = export.data.as_ref().expect("export must return data");
    let raw = data["bytes_utf8"].as_str().expect("bytes_utf8 string");
    assert_eq!(
        raw, "{\"$schema\":\"http://json-schema.org/draft-07/schema#\"}\n",
        "ontology.export must surface daemon bytes byte-identically (no reparse / reformat)",
    );

    let log = calls.log.lock().unwrap().clone();
    assert_eq!(log.len(), 3, "{log:?}");
    assert_eq!(log[0], ("GET".into(), "/v1/ontology/types".into()));
    assert_eq!(
        log[1],
        ("GET".into(), "/v1/ontology/types/object/Person".into())
    );
    assert_eq!(
        log[2],
        (
            "GET".into(),
            "/v1/ontology/export/jsonschema/object/Person".into()
        )
    );

    // Missing required kind must reject before the network call.
    let bad = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::OntologyDescribe,
            params: json!({"name": "Person"}),
        })
        .await;
    assert!(!bad.ok, "missing kind must be a typed error");

    let bad_export = bridge
        .dispatch(ActRequest {
            schema_version: SCHEMA_VERSION.into(),
            action: Action::OntologyExport,
            params: json!({"name": "Person"}),
        })
        .await;
    assert!(!bad_export.ok, "missing format must be a typed error");
}
