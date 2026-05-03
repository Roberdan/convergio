//! # convergio-executor — Layer 4 (basic)
//!
//! Dispatcher loop. Picks `pending` tasks whose wave is ready and asks
//! Layer 3 ([`convergio_lifecycle::Supervisor`]) to spawn agents for
//! them.
//!
//! ## Design
//!
//! One executor loop, ticking on a configurable interval. It is **not**
//! a workflow engine — it just translates Layer 1 state into Layer 3
//! spawn calls + Layer 1 state transitions. If the loop dies, no state
//! is lost (it lives in Layer 1).
//!
//! ## MVP behaviour (deterministic, no LLM)
//!
//! For each `pending` task whose wave is ready (no earlier-wave task
//! is still open):
//!
//! 1. Spawn `command` (default `/bin/echo`) using the supplied
//!    [`SpawnTemplate`] with the task id as the only arg.
//! 2. Move the task to `in_progress`, with `agent_id` set to the
//!    spawned process id.
//!
//! What the agent does once spawned is the agent's problem. The
//! executor only owns dispatch.
//!
//! ## Quickstart
//!
//! ```no_run
//! use convergio_db::Pool;
//! use convergio_durability::{init, Durability};
//! use convergio_executor::{Executor, SpawnTemplate};
//! use convergio_lifecycle::Supervisor;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = Pool::connect("sqlite://./state.db").await?;
//! init(&pool).await?;
//! convergio_lifecycle::init(&pool).await?;
//! let dur = Durability::new(pool.clone());
//! let sup = Supervisor::new(pool);
//! let exec = Executor::new(dur, sup, SpawnTemplate::default());
//! let dispatched = exec.tick().await?;
//! println!("dispatched {dispatched} tasks");
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

mod defaults;
mod error;
mod executor;

pub use defaults::{RunnerDefaults, SpawnTemplate};
pub use error::{ExecutorError, Result};
pub use executor::{spawn_loop, Executor, ExecutorHandle};
