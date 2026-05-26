//! Basic interpreter tests for the workflow engine.

use chrono::{Duration, Utc};
use convergio_ops::{
    ActionSpec, ConditionExpr, EngineEvent, GatewayKind, HumanTaskSpec, NodeKind, NodeSpec,
    TimerSpec, WorkflowEngine, WorkflowSpec,
};
use serde_json::json;

#[test]
fn sequence_creates_action_work_item_then_completes() {
    let spec = WorkflowSpec {
        start: "start".into(),
        nodes: vec![
            NodeSpec {
                id: "start".into(),
                kind: NodeKind::Start,
                next: vec!["a1".into()],
            },
            NodeSpec {
                id: "a1".into(),
                kind: NodeKind::Action(ActionSpec {
                    name: "demo.action".into(),
                    input: json!({"x": 1}),
                    compensation: None,
                }),
                next: vec!["end".into()],
            },
            NodeSpec {
                id: "end".into(),
                kind: NodeKind::End,
                next: vec![],
            },
        ],
    };

    let engine = WorkflowEngine::new(spec);
    let now = Utc::now();
    let state0 = engine.start("i1".into(), json!({}));

    let out1 = engine.tick(state0, now);
    assert!(out1
        .events
        .iter()
        .any(|e| matches!(e, EngineEvent::WorkItemCreated { .. })));

    let work_id = out1.state.work_items[0].id.clone();
    let state = engine.complete_work_item(out1.state, &work_id, true);
    let out2 = engine.tick(state, now);
    assert!(out2
        .events
        .iter()
        .any(|e| matches!(e, EngineEvent::Completed)));
}

#[test]
fn parallel_fork_and_join_waits_for_all_paths() {
    let spec = WorkflowSpec {
        start: "start".into(),
        nodes: vec![
            NodeSpec {
                id: "start".into(),
                kind: NodeKind::Start,
                next: vec!["fork".into()],
            },
            NodeSpec {
                id: "fork".into(),
                kind: NodeKind::ParallelGateway {
                    kind: GatewayKind::Fork,
                },
                next: vec!["h1".into(), "h2".into()],
            },
            NodeSpec {
                id: "h1".into(),
                kind: NodeKind::HumanTask(HumanTaskSpec {
                    title: "one".into(),
                    escalation_after_ms: None,
                    escalation_action: None,
                }),
                next: vec!["join".into()],
            },
            NodeSpec {
                id: "h2".into(),
                kind: NodeKind::HumanTask(HumanTaskSpec {
                    title: "two".into(),
                    escalation_after_ms: None,
                    escalation_action: None,
                }),
                next: vec!["join".into()],
            },
            NodeSpec {
                id: "join".into(),
                kind: NodeKind::ParallelGateway {
                    kind: GatewayKind::Join,
                },
                next: vec!["end".into()],
            },
            NodeSpec {
                id: "end".into(),
                kind: NodeKind::End,
                next: vec![],
            },
        ],
    };

    let engine = WorkflowEngine::new(spec);
    let now = Utc::now();
    let out0 = engine.tick(engine.start("i".into(), json!({})), now);
    assert_eq!(out0.state.work_items.len(), 2);

    let w1 = out0.state.work_items[0].id.clone();
    let w2 = out0.state.work_items[1].id.clone();

    let out1 = engine.tick(engine.complete_work_item(out0.state, &w1, true), now);
    assert!(!out1
        .events
        .iter()
        .any(|e| matches!(e, EngineEvent::Completed)));

    let out2 = engine.tick(engine.complete_work_item(out1.state, &w2, true), now);
    assert!(out2
        .events
        .iter()
        .any(|e| matches!(e, EngineEvent::Completed)));
}

#[test]
fn exclusive_gateway_routes_by_condition() {
    let spec = WorkflowSpec {
        start: "start".into(),
        nodes: vec![
            NodeSpec {
                id: "start".into(),
                kind: NodeKind::Start,
                next: vec!["gw".into()],
            },
            NodeSpec {
                id: "gw".into(),
                kind: NodeKind::ExclusiveGateway {
                    routes: vec![
                        convergio_ops::ExclusiveRoute {
                            when: ConditionExpr::Eq {
                                key: "path".into(),
                                value: json!("a"),
                            },
                            to: "a".into(),
                        },
                        convergio_ops::ExclusiveRoute {
                            when: ConditionExpr::Always,
                            to: "b".into(),
                        },
                    ],
                },
                next: vec!["a".into(), "b".into()],
            },
            NodeSpec {
                id: "a".into(),
                kind: NodeKind::End,
                next: vec![],
            },
            NodeSpec {
                id: "b".into(),
                kind: NodeKind::End,
                next: vec![],
            },
        ],
    };

    let engine = WorkflowEngine::new(spec);
    let now = Utc::now();
    let out = engine.tick(engine.start("i".into(), json!({"path": "a"})), now);
    assert!(out
        .events
        .iter()
        .any(|e| matches!(e, EngineEvent::Completed)));
}

#[test]
fn timer_waits_then_advances() {
    let spec = WorkflowSpec {
        start: "start".into(),
        nodes: vec![
            NodeSpec {
                id: "start".into(),
                kind: NodeKind::Start,
                next: vec!["t".into()],
            },
            NodeSpec {
                id: "t".into(),
                kind: NodeKind::Timer(TimerSpec { after_ms: 10 }),
                next: vec!["end".into()],
            },
            NodeSpec {
                id: "end".into(),
                kind: NodeKind::End,
                next: vec![],
            },
        ],
    };

    let engine = WorkflowEngine::new(spec);
    let now = Utc::now();
    let out1 = engine.tick(engine.start("i".into(), json!({})), now);
    assert!(!out1
        .events
        .iter()
        .any(|e| matches!(e, EngineEvent::Completed)));

    let out2 = engine.tick(out1.state, now + Duration::milliseconds(20));
    assert!(out2
        .events
        .iter()
        .any(|e| matches!(e, EngineEvent::Completed)));
}
