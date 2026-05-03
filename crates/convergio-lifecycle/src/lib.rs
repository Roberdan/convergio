//! # convergio-lifecycle — Layer 3
//!
//! Spawn, supervise, heartbeat and reap long-running agent processes.
//!
//! Layer 3 owns the `agent_processes` table. It launches arbitrary
//! executables (Claude Code session, Python script, shell command),
//! tracks their PID + exit code, and exposes a heartbeat endpoint that
//! upper layers can use to detect dead agents.
//!
//! ## What it is NOT
//!
//! - **Not** systemd or launchd — we don't manage system services.
//! - **Not** a sandbox — agents run with the daemon's privileges.
//! - **Not** Kubernetes — no resource limits, no scheduling, no networking.
//!
//! ## Quickstart
//!
//! ```no_run
//! use convergio_lifecycle::{init, Supervisor, SpawnSpec};
//! use convergio_db::Pool;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = Pool::connect("sqlite://./state.db").await?;
//! init(&pool).await?;
//! let sup = Supervisor::new(pool);
//! let process = sup.spawn(SpawnSpec {
//!     kind: "shell".into(),
//!     command: "/bin/echo".into(),
//!     args: vec!["hello".into()],
//!     env: vec![],
//!     plan_id: None,
//!     task_id: None,
//!     cwd: None,
//!     stdin_payload: None,
//! }).await?;
//! sup.heartbeat(&process.id).await?;
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

mod error;
mod migrate;
mod model;
mod supervisor;
pub mod watcher;

pub use error::{LifecycleError, Result};
pub use migrate::init;
pub use model::{AgentProcess, ProcessStatus, SpawnSpec};
pub use supervisor::Supervisor;
