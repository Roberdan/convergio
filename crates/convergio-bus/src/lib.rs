//! # convergio-bus — Layer 2
//!
//! Persistent agent-to-agent message bus, scoped to a single plan.
//!
//! ## Model
//!
//! | Field        | Notes |
//! |--------------|-------|
//! | `topic`      | Free-form. Convention: `task.done`, `plan.invalidated`, `agent:agent-id` for direct |
//! | `sender`     | Agent id, or `None` for system-emitted messages |
//! | `payload`    | Canonical JSON — interpretation is the consumer's job |
//! | `consumed_at`| `None` until [`Bus::ack`] is called by the consumer |
//!
//! Messages are usually scoped per `plan_id`: a consumer subscribed to
//! plan A never sees messages from plan B. The `system.*` topic family
//! (ADR-0025) is the narrow exception — those messages have
//! `plan_id IS NULL` and are written via [`Bus::publish_system`] /
//! read via [`Bus::poll_system`]. Everything else stays plan-scoped.
//!
//! ## Delivery semantics
//!
//! - **At-least-once**: a consumer may see the same message twice if it
//!   crashes between [`Bus::poll`] and [`Bus::ack`]. Consumers must be
//!   idempotent.
//! - **Persistent**: messages live in the DB until acked. A consumer can
//!   crash and restart without losing in-flight messages.
//! - **Per-plan FIFO**: messages within a `(plan_id, topic)` are
//!   delivered in `seq` order.
//!
//! ## Quickstart
//!
//! ```no_run
//! use convergio_bus::{init, Bus, NewMessage};
//! use convergio_db::Pool;
//! use serde_json::json;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = Pool::connect("sqlite://./state.db").await?;
//! init(&pool).await?;
//! let bus = Bus::new(pool);
//! bus.publish(NewMessage {
//!     plan_id: "plan-uuid".into(),
//!     topic: "task.done".into(),
//!     sender: Some("agent-1".into()),
//!     payload: json!({"task_id": "..."}),
//! }).await?;
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

mod bus;
mod bus_inspection;
mod bus_system;
mod error;
mod migrate;
mod model;

pub use bus::Bus;
pub use error::{BusError, Result};
pub use migrate::init;
pub use model::{AgentLastTopic, Message, NewMessage, NewSystemMessage, TopicSummary};
