//! LLM gateway MVP E2E: caching, egress redaction, output-schema validation,
//! and W3C-PROV bundle emission.

mod common;

use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

/// Serializes tests: they share process-wide `LLM_GATEWAY_*` env vars.
fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Provider stub that echoes the received prompt and records the last body.
async fn start_stub_provider(counter: Arc<AtomicUsize>, last_prompt: Arc<Mutex<String>>) -> String {
    async fn complete(
        axum::extract::State((counter, last_prompt)): axum::extract::State<(
            Arc<AtomicUsize>,
            Arc<Mutex<String>>,
        )>,
        Json(req): Json<Value>,
    ) -> Json<Value> {
        counter.fetch_add(1, Ordering::SeqCst);
        let prompt = req.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
        *last_prompt.lock().await = prompt.to_string();
        Json(json!({
            "output": format!("echo: {prompt}"),
            "usage": {"input_tokens": 10, "output_tokens": 3}
        }))
    }

    let app = Router::new()
        .route("/v1/complete", post(complete))
        .with_state((counter, last_prompt));
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    format!("http://{addr}")
}

fn set_provider_env(stub: &str) {
    std::env::set_var(
        "LLM_GATEWAY_PURPOSE_MODEL_ALLOWLIST_JSON",
        r#"{"summarize":["azure:gpt-4o-mini"]}"#,
    );
    std::env::set_var("LLM_GATEWAY_AZURE_OPENAI_URL", stub);
}

#[tokio::test]
async fn gateway_caches_by_prompt_model_and_retrieval_set_and_emits_provenance_on_hits() {
    let _guard = env_lock().lock().await;
    let (base, _pool, _dir) = common::boot().await;

    let counter = Arc::new(AtomicUsize::new(0));
    let last_prompt = Arc::new(Mutex::new(String::new()));
    let stub = start_stub_provider(counter.clone(), last_prompt).await;
    set_provider_env(&stub);

    let client = common::client();
    let req = json!({
        "purpose": "summarize",
        "model_id": "azure:gpt-4o-mini",
        "prompt": "hello",
        "retrieval_set_hash": "none",
        "max_output_tokens": 16,
        "cache": true
    });

    let first: Value = client
        .post(format!("{base}/v1/llm-gateway/call"))
        .json(&req)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(first["meta"]["cache_hit"], false);
    assert_eq!(first["result"]["output"], "echo: hello");
    // W3C-PROV bundle is emitted with activity/agent/entity + relations.
    let prov = &first["provenance"];
    assert!(prov["activity"].is_array() && prov["agent"].is_array() && prov["entity"].is_array());
    assert!(prov["wasGeneratedBy"].is_array() && prov["wasAssociatedWith"].is_array());
    assert_eq!(prov["activity"][0]["kind"], "llm.gateway.call");
    assert!(prov["agent"][0]["label"]
        .as_str()
        .unwrap()
        .contains("prompt_hash="));

    let second: Value = client
        .post(format!("{base}/v1/llm-gateway/call"))
        .json(&req)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(second["meta"]["cache_hit"], true);
    assert_eq!(second["result"]["output"], "echo: hello");
    // PROV bundle is emitted on the cache hit too.
    assert!(second["provenance"]["wasGeneratedBy"].is_array());

    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "provider should be called once"
    );
}

#[tokio::test]
async fn gateway_redacts_pii_in_outbound_prompt() {
    let _guard = env_lock().lock().await;
    let (base, _pool, _dir) = common::boot().await;

    let counter = Arc::new(AtomicUsize::new(0));
    let last_prompt = Arc::new(Mutex::new(String::new()));
    let stub = start_stub_provider(counter.clone(), last_prompt.clone()).await;
    set_provider_env(&stub);

    let client = common::client();
    let req = json!({
        "purpose": "summarize",
        "model_id": "azure:gpt-4o-mini",
        "prompt": "email me at jane.doe@example.com about sk-ABCDEF0123456789",
        "retrieval_set_hash": "none",
        "max_output_tokens": 16,
        "cache": false
    });

    let body: Value = client
        .post(format!("{base}/v1/llm-gateway/call"))
        .json(&req)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    // The provider only ever saw the redacted prompt.
    let seen = last_prompt.lock().await.clone();
    assert!(seen.contains("[REDACTED_EMAIL]"), "prompt seen: {seen}");
    assert!(seen.contains("[REDACTED_SECRET]"), "prompt seen: {seen}");
    assert!(!seen.contains("jane.doe@example.com"));

    let redactions = body["egress"]["redactions"].as_array().unwrap();
    assert!(redactions.contains(&json!("email")));
    assert!(redactions.contains(&json!("secret")));
}

#[tokio::test]
async fn gateway_refuses_on_output_schema_mismatch() {
    let _guard = env_lock().lock().await;
    let (base, _pool, _dir) = common::boot().await;

    let counter = Arc::new(AtomicUsize::new(0));
    let last_prompt = Arc::new(Mutex::new(String::new()));
    let stub = start_stub_provider(counter.clone(), last_prompt).await;
    set_provider_env(&stub);

    let client = common::client();
    let req = json!({
        "purpose": "summarize",
        "model_id": "azure:gpt-4o-mini",
        "prompt": "hello",
        "retrieval_set_hash": "none",
        "max_output_tokens": 16,
        "cache": false,
        "expected_output_schema": {
            "type": "object",
            "required": ["verdict"],
            "properties": {"verdict": {"type": "string"}}
        }
    });

    let resp = client
        .post(format!("{base}/v1/llm-gateway/call"))
        .json(&req)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 400);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["error"]["code"], "output_schema_mismatch");
}
