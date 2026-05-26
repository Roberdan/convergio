//! Interpreter tests focused on side effects (escalation + compensation).

use chrono::{Duration, Utc};
use convergio_ops::{
    ActionSpec, CompensationSpec, EngineEvent, HumanTaskSpec, NodeKind, NodeSpec, WorkflowEngine,
    WorkflowSpec,
};
use serde_json::json;

#[test]
fn human_task_escalates_once_after_threshold() {
    let spec = WorkflowSpec {
        start: "start".into(),
        nodes: vec![
            NodeSpec {
                id: "start".into(),
                kind: NodeKind::Start,
                next: vec!["h".into()],
            },
            NodeSpec {
                id: "h".into(),
                kind: NodeKind::HumanTask(HumanTaskSpec {
                    title: "approve".into(),
                    escalation_after_ms: Some(5),
                    escalation_action: Some(ActionSpec {
                        name: "ops.escalate".into(),
                        input: json!({"severity": "high"}),
                        compensation: None,
                    }),
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
    let out0 = engine.tick(engine.start("i".into(), json!({})), now);

    let later = now + Duration::milliseconds(10);
    let out1 = engine.tick(out0.state, later);
    assert!(out1
        .events
        .iter()
        .any(|e| matches!(e, EngineEvent::EscalationEmitted { .. })));

    let out2 = engine.tick(out1.state, later + Duration::milliseconds(10));
    let escalations = out2
        .events
        .iter()
        .filter(|e| matches!(e, EngineEvent::EscalationEmitted { .. }))
        .count();
    assert_eq!(escalations, 0);
}

#[test]
fn compensation_emits_actions_in_reverse_completion_order() {
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
                    name: "ops.do1".into(),
                    input: json!({}),
                    compensation: Some(CompensationSpec {
                        name: "ops.undo1".into(),
                        input: json!({"n": 1}),
                    }),
                }),
                next: vec!["a2".into()],
            },
            NodeSpec {
                id: "a2".into(),
                kind: NodeKind::Action(ActionSpec {
                    name: "ops.do2".into(),
                    input: json!({}),
                    compensation: Some(CompensationSpec {
                        name: "ops.undo2".into(),
                        input: json!({"n": 2}),
                    }),
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

    let out0 = engine.tick(engine.start("i".into(), json!({})), now);
    let w1 = out0
        .state
        .work_items
        .iter()
        .find(|w| w.node_id == "a1")
        .unwrap()
        .id
        .clone();

    let out1 = engine.tick(engine.complete_work_item(out0.state, &w1, true), now);
    let w2 = out1
        .state
        .work_items
        .iter()
        .find(|w| w.node_id == "a2")
        .unwrap()
        .id
        .clone();

    let out2 = engine.tick(engine.complete_work_item(out1.state, &w2, true), now);
    assert!(out2
        .events
        .iter()
        .any(|e| matches!(e, EngineEvent::Completed)));

    let comp = engine.begin_compensation(out2.state, now);
    let created_ids: Vec<String> = comp
        .events
        .iter()
        .filter_map(|e| match e {
            EngineEvent::WorkItemCreated { work_item_id } => Some(work_item_id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(created_ids.len(), 2);

    let names: Vec<String> = created_ids
        .iter()
        .map(|id| {
            let w = comp.state.work_item(id).unwrap();
            match &w.kind {
                convergio_ops::WorkItemKind::Action { name, .. } => name.clone(),
                _ => panic!("expected action"),
            }
        })
        .collect();

    assert_eq!(
        names,
        vec!["ops.undo2".to_string(), "ops.undo1".to_string()]
    );
}
