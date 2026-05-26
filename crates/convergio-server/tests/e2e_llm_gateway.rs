//! LLM gateway MVP E2E.

mod common;

use axum::{routing::post, Json, Router};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::net::TcpListener;

async fn start_stub_provider(counter: Arc<AtomicUsize>) -> String {
    async fn complete(
        counter: axum::extract::State<Arc<AtomicUsize>>,
        Json(req): Json<Value>,
    ) -> Json<Value> {
        counter.fetch_add(1, Ordering::SeqCst);
        let prompt = req.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
        Json(json!({
            "output": format!("echo: {prompt}"),
            "usage": {"input_tokens": 10, "output_tokens": 3}
        }))
    }

    let app = Router::new()
        .route("/v1/complete", post(complete))
        .with_state(counter);
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    format!("http://{addr}")
}

#[tokio::test]
async fn gateway_caches_by_prompt_model_and_retrieval_set_and_emits_provenance_on_hits() {
    let (base, _pool, _dir) = common::boot().await;

    let counter = Arc::new(AtomicUsize::new(0));
    let stub = start_stub_provider(counter.clone()).await;

    std::env::set_var(
        "LLM_GATEWAY_PURPOSE_MODEL_ALLOWLIST_JSON",
        r#"{"summarize":["azure:gpt-4o-mini"]}"#,
    );
    std::env::set_var("LLM_GATEWAY_AZURE_OPENAI_URL", &stub);

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
    assert_eq!(first["provenance"]["cache_hit"], false);
    assert_eq!(first["result"]["output"], "echo: hello");

    let second: Value = client
        .post(format!("{base}/v1/llm-gateway/call"))
        .json(&req)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(second["provenance"]["cache_hit"], true);
    assert_eq!(second["result"]["output"], "echo: hello");

    assert_eq!(
        counter.load(Ordering::SeqCst),
        1,
        "provider should be called once"
    );
}
