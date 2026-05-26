//! Task context packet E2E tests.

use convergio_bus::{Bus, NewMessage};
use convergio_db::Pool;
use convergio_durability::{init, NewAgent, NewPlan, NewTask};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::net::TcpListener;

#[tokio::test]
async fn context_packet_collects_task_state_messages_agents_and_agent_docs() {
    let dir = tempdir().unwrap();
    let db_path = dir.path().join("state.db");
    let pool = Pool::connect(&format!("sqlite://{}", db_path.display()))
        .await
        .unwrap();
    init(&pool).await.unwrap();
    convergio_bus::init(&pool).await.unwrap();
    convergio_lifecycle::init(&pool).await.unwrap();
    let dur = convergio_durability::Durability::new(pool.clone());
    let bus = Bus::new(pool.clone());

    let plan = dur
        .create_plan(NewPlan {
            title: "ship context".into(),
            description: Some("keep worker prompt small".into()),
            project: Some("convergio-local".into()),
        })
        .await
        .unwrap();
    let task = dur
        .create_task(
            &plan.id,
            NewTask {
                title: "write context packet".into(),
                description: Some("include relevant state only".into()),
                wave: 1,
                sequence: 1,
                evidence_required: vec!["code".into()],
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            },
        )
        .await
        .unwrap();
    dur.attach_evidence(&task.id, "code", json!({"commit": "abc"}), Some(0))
        .await
        .unwrap();
    dur.register_agent(NewAgent {
        id: "agent-a".into(),
        kind: "copilot".into(),
        name: None,
        host: None,
        capabilities: vec!["code".into()],
        metadata: json!({}),
    })
    .await
    .unwrap();
    bus.publish(NewMessage {
        plan_id: plan.id.clone(),
        topic: format!("task:{}", task.id),
        sender: Some("agent-a".into()),
        payload: json!({"note": "ready"}),
    })
    .await
    .unwrap();

    let workspace = dir.path().join("repo");
    let src = workspace.join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(workspace.join("AGENTS.md"), "root rules").unwrap();
    std::fs::write(src.join("AGENTS.md"), "src rules").unwrap();
    std::fs::write(src.join("lib.rs"), "fn main() {}").unwrap();

    let state = AppState {
        durability: Arc::new(dur),
        ops: Arc::new(convergio_ops::Ops::new(pool.clone())),
        bus: Arc::new(bus),
        supervisor: Arc::new(Supervisor::new(pool.clone())),
        graph: Arc::new(convergio_graph::Store::new(pool.clone())),
        embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
        embedder: Arc::new(convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8)),
        fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
        fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
        .await
        .unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router(state)).await.unwrap();
    });
    let base = format!("http://{addr}");

    let packet: Value = reqwest::Client::new()
        .post(format!("{base}/v1/tasks/{}/context", task.id))
        .json(&json!({"workspace_path": src.join("lib.rs")}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();

    assert_eq!(packet["schema_version"], "1");
    assert_eq!(packet["plan"]["id"], plan.id);
    assert_eq!(packet["task"]["id"], task.id);
    assert_eq!(packet["evidence"].as_array().unwrap().len(), 1);
    assert_eq!(packet["messages"][0]["payload"]["note"], "ready");
    assert_eq!(packet["agents"][0]["id"], "agent-a");
    assert_eq!(packet["agent_instructions"][0]["content"], "src rules");
    assert_eq!(packet["agent_instructions"][1]["content"], "root rules");

    let invalid: Value = reqwest::Client::new()
        .post(format!("{base}/v1/tasks/{}/context", task.id))
        .json(&json!({"message_limit": 101}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(invalid["error"]["code"], "invalid_context_limit");
}
