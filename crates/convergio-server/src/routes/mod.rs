//! HTTP route modules — one file per resource.

pub mod agent_registry;
pub mod agents;
pub mod api_actions;
pub mod audit;
pub mod capabilities;
pub mod context;
pub mod crdt;
pub mod dispatch;
pub mod embed;
pub mod evidence;
pub mod gate_preconditions;
pub mod graph;
pub mod health;
pub(crate) mod limits;
pub mod messages;
pub mod ops;
pub mod plans;
pub mod pr_links;
pub mod solve;
pub mod status;
pub mod system_messages;
pub mod tasks;
pub mod telemetry;
pub mod validate;
pub mod workspace;
