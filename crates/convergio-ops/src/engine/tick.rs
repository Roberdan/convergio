//! Tick logic for the workflow interpreter.

use super::{EngineEvent, EngineTickOutcome, WorkflowEngine};
use crate::spec::{ExclusiveRoute, GatewayKind, NodeId, NodeKind};
use crate::state::{EngineCursor, WorkItem, WorkItemKind, WorkItemStatus, WorkflowInstanceState};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::collections::VecDeque;
use uuid::Uuid;

/// Advance the instance state by processing all ready cursors.
pub(super) fn tick(
    engine: &WorkflowEngine,
    mut state: WorkflowInstanceState,
    now: DateTime<Utc>,
) -> EngineTickOutcome {
    let mut events: Vec<EngineEvent> = Vec::new();

    // Escalations: scan pending human tasks for due-at exceed.
    for w in &mut state.work_items {
        if w.status != WorkItemStatus::Pending || w.escalated {
            continue;
        }
        let Some(kind) = engine.kinds.get(&w.node_id) else {
            continue;
        };
        let NodeKind::HumanTask(spec) = kind else {
            continue;
        };
        let Some(after_ms) = spec.escalation_after_ms else {
            continue;
        };
        let due = w.created_at + Duration::milliseconds(after_ms);
        if now >= due {
            if let Some(action) = spec.escalation_action.clone() {
                let action_kind = WorkItemKind::Action {
                    name: action.name,
                    input: action.input,
                };
                w.escalated = true;
                events.push(EngineEvent::EscalationEmitted {
                    node_id: w.node_id.clone(),
                    action: action_kind,
                });
            }
        }
    }

    let mut q: VecDeque<EngineCursor> = state.cursors.drain(..).collect();
    let mut new_cursors: Vec<EngineCursor> = Vec::new();

    let mut steps: usize = 0;
    const MAX_STEPS: usize = 10_000;

    let mut failed = false;

    while let Some(cursor) = q.pop_front() {
        steps += 1;
        if steps > MAX_STEPS {
            new_cursors.push(cursor);
            break;
        }

        let node_id = cursor.node_id.clone();
        let kind = match engine.kinds.get(&node_id) {
            Some(k) => k.clone(),
            None => {
                new_cursors.push(cursor);
                continue;
            }
        };

        match kind {
            NodeKind::Start => enqueue_next(engine, &node_id, None, &mut q),
            NodeKind::End => {}
            NodeKind::Timer(t) => {
                let due_at = cursor
                    .due_at
                    .unwrap_or_else(|| now + Duration::milliseconds(t.after_ms));
                if now >= due_at {
                    enqueue_next(engine, &node_id, None, &mut q);
                } else {
                    new_cursors.push(EngineCursor {
                        due_at: Some(due_at),
                        ..cursor
                    });
                }
            }
            NodeKind::Action(action) => {
                if let Some(item) = find_work_item(&state.work_items, &node_id) {
                    match item.status {
                        WorkItemStatus::Completed => {
                            engine.record_completed_action(&mut state, node_id.clone(), action);
                            enqueue_next(engine, &node_id, None, &mut q);
                        }
                        WorkItemStatus::Failed => {
                            events.push(EngineEvent::WorkItemFailed {
                                work_item_id: item.id.clone(),
                                node_id: node_id.clone(),
                            });
                            failed = true;
                            q.clear();
                            new_cursors.clear();
                        }
                        WorkItemStatus::Pending => new_cursors.push(cursor),
                    }
                } else {
                    let item = WorkItem {
                        id: Uuid::new_v4().to_string(),
                        node_id: node_id.clone(),
                        kind: WorkItemKind::Action {
                            name: action.name,
                            input: action.input,
                        },
                        status: WorkItemStatus::Pending,
                        created_at: now,
                        due_at: None,
                        escalated: false,
                    };
                    events.push(EngineEvent::WorkItemCreated {
                        work_item_id: item.id.clone(),
                    });
                    state.work_items.push(item);
                    new_cursors.push(cursor);
                }
            }
            NodeKind::HumanTask(h) => {
                if let Some(item) = find_work_item(&state.work_items, &node_id) {
                    match item.status {
                        WorkItemStatus::Completed => enqueue_next(engine, &node_id, None, &mut q),
                        WorkItemStatus::Failed => {
                            events.push(EngineEvent::WorkItemFailed {
                                work_item_id: item.id.clone(),
                                node_id: node_id.clone(),
                            });
                            failed = true;
                            q.clear();
                            new_cursors.clear();
                        }
                        WorkItemStatus::Pending => new_cursors.push(cursor),
                    }
                } else {
                    let item = WorkItem {
                        id: Uuid::new_v4().to_string(),
                        node_id: node_id.clone(),
                        kind: WorkItemKind::Human { title: h.title },
                        status: WorkItemStatus::Pending,
                        created_at: now,
                        due_at: None,
                        escalated: false,
                    };
                    events.push(EngineEvent::WorkItemCreated {
                        work_item_id: item.id.clone(),
                    });
                    state.work_items.push(item);
                    new_cursors.push(cursor);
                }
            }
            NodeKind::ParallelGateway {
                kind: GatewayKind::Fork,
            } => enqueue_next(engine, &node_id, Some(node_id.clone()), &mut q),
            NodeKind::ParallelGateway {
                kind: GatewayKind::Join,
            } => {
                record_join_arrival(&mut state, &node_id, cursor.arrived_from);
                let incoming = engine.incoming.get(&node_id).cloned().unwrap_or_default();
                let have = state.join_memory.get(&node_id).cloned().unwrap_or_default();
                if !incoming.is_empty() && incoming.iter().all(|n| have.contains(n)) {
                    state.join_memory.remove(&node_id);
                    enqueue_next(engine, &node_id, Some(node_id.clone()), &mut q);
                }
            }
            NodeKind::ExclusiveGateway { routes } => {
                let chosen =
                    choose_route(&routes, &state.context).or_else(|| first_next(engine, &node_id));
                if let Some(to) = chosen {
                    q.push_back(EngineCursor {
                        node_id: to,
                        arrived_from: Some(node_id.clone()),
                        due_at: None,
                    });
                } else {
                    new_cursors.push(cursor);
                }
            }
        }
    }

    state.cursors = new_cursors;

    if !failed && state.cursors.is_empty() && !state.has_pending_work() {
        events.push(EngineEvent::Completed);
    }

    EngineTickOutcome { state, events }
}

fn record_join_arrival(
    state: &mut WorkflowInstanceState,
    join_node: &str,
    arrived_from: Option<NodeId>,
) {
    let arrived = arrived_from.unwrap_or_default();
    if arrived.is_empty() {
        return;
    }
    let seen = state.join_memory.entry(join_node.to_string()).or_default();
    if !seen.contains(&arrived) {
        seen.push(arrived);
    }
}

fn choose_route(routes: &[ExclusiveRoute], ctx: &Value) -> Option<NodeId> {
    for r in routes {
        if r.when.eval(ctx) {
            return Some(r.to.clone());
        }
    }
    None
}

fn first_next(engine: &WorkflowEngine, from: &str) -> Option<NodeId> {
    engine.outgoing.get(from).and_then(|v| v.first()).cloned()
}

fn enqueue_next(
    engine: &WorkflowEngine,
    from: &str,
    arrived_from: Option<NodeId>,
    q: &mut VecDeque<EngineCursor>,
) {
    let Some(next) = engine.outgoing.get(from) else {
        return;
    };
    for to in next {
        q.push_back(EngineCursor {
            node_id: to.clone(),
            arrived_from: arrived_from.clone().or_else(|| Some(from.to_string())),
            due_at: None,
        });
    }
}

fn find_work_item<'a>(items: &'a [WorkItem], node_id: &str) -> Option<&'a WorkItem> {
    items
        .iter()
        .find(|w| w.node_id == node_id && w.status == WorkItemStatus::Pending)
        .or_else(|| items.iter().find(|w| w.node_id == node_id))
}
