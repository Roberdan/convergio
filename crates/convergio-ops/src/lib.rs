//! Workflow & Operations Engine (Ontology Platform W8).
//!
//! Owns workflow definitions + workflow instances as persisted, bitemporal
//! entities, plus a small interpreter for a BPMN-2.0 subset.

#![forbid(unsafe_code)]

mod facade;
mod migrate;

pub mod engine;
pub mod model;
pub mod spec;
pub mod state;
pub mod store;

pub use facade::Ops;
pub use migrate::init;

pub use engine::{EngineEvent, EngineTickOutcome, WorkflowEngine};
pub use model::{OpsWorkflow, OpsWorkflowInstance, OpsWorkflowInstanceStatus};
pub use spec::{
    ActionSpec, CompensationSpec, ConditionExpr, ExclusiveRoute, GatewayKind, HumanTaskSpec,
    NodeId, NodeKind, NodeSpec, TimerSpec, WorkflowSpec,
};
pub use state::{
    CompletedAction, EngineCursor, WorkItem, WorkItemKind, WorkItemStatus, WorkflowInstanceState,
};
