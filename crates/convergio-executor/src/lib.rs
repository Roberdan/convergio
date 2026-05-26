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
//! ## Two dispatch paths
//!
//! For each `pending` task whose wave is ready (no earlier-wave task
//! is still open), the executor picks one of two paths based on
//! `task.runner_kind` and `CONVERGIO_EXECUTOR_USE_RUNNER`:
//!
//! 1. **Legacy `SpawnTemplate` (MVP / smoke-test path).** Selected
//!    when `runner_kind` is `None` and the env var is unset. Spawns
//!    `command` (default `/bin/echo`) with the template's args plus
//!    the task id appended, then moves the task to `in_progress`
//!    with `agent_id` set to the spawned process id.
//! 2. **Runner-based (production path, ADR-0034).** Selected when
//!    either `runner_kind` is set on the task row or the env var is
//!    set on the daemon. Pre-creates a git worktree under
//!    `<repo>/.claude/worktrees/agent-<task7>`, picks the runner
//!    (`claude:sonnet` / `copilot:gpt-5.2` / custom from
//!    `~/.convergio/runners.toml`), assembles the prompt + flags via
//!    [`convergio_runner`], spawns it through Layer 3
//!    [`Supervisor`](convergio_lifecycle::Supervisor), and starts a
//!    heartbeat sidecar tied to the spawned PID.
//!
//! Both paths atomically claim the task before spawning (W1-B).
//! What the agent does once spawned is the agent's problem; the
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
mod dispatch_choice;
mod error;
mod executor;
mod graph_seed;
pub mod guards;
mod guards_format;
mod heartbeat;
mod holders;
mod run_loop;
pub mod worktree;

pub use defaults::{RunnerDefaults, SpawnTemplate};
pub use error::{ExecutorError, Result};
pub use executor::Executor;
pub use run_loop::{spawn_loop, ExecutorHandle};
