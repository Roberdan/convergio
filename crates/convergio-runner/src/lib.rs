//! # convergio-runner — vendor-CLI runners for Convergio agents
//!
//! Spawns work sessions against the user's *already authenticated*
//! vendor CLIs (`claude`, `gh copilot`). The Convergio constraint
//! (ADR-0032): **never** call the raw Anthropic / OpenAI HTTP APIs.
//! All cost, auth and rate-limit policy lives in the CLI the operator
//! has already paid for and signed in to.
//!
//! ## Why
//!
//! Operators run Convergio with a Claude Max plan or a GitHub Copilot
//! subscription — they have *already* solved auth + budget at the
//! vendor level. Re-invoking those credentials over the API would
//! double-bill, leak secrets through env vars, and split the
//! observability story across two systems. The CLI shells out to
//! something the operator can `man`, can audit, and can replace.
//!
//! ## Surface
//!
//! - [`RunnerKind`] selects the vendor + model (`claude:sonnet`,
//!   `claude:opus`, `copilot:gpt-5.2`, `copilot:claude-opus`, ...).
//! - [`Runner::prepare`] builds a [`std::process::Command`] for a
//!   given [`SpawnContext`]. It does *not* execute it — that is the
//!   caller's choice (the executor spawns + supervises; tests just
//!   assert on the constructed argv + stdin).
//! - [`prompt::build`] composes the prompt: task metadata + graph
//!   context-pack (when available) + the agent contract
//!   (heartbeats, evidence, transition, PR convention).
//!
//! ## What is *not* in this crate
//!
//! - Process supervision — that lives in `convergio-lifecycle::Supervisor`.
//! - HTTP / API key handling — see ADR-0032.
//! - The executor's routing decision — that lives in
//!   `convergio-executor`; the runner just executes the routed kind.

#![forbid(unsafe_code)]

mod command;
mod error;
mod kind;
mod profile;
pub mod prompt;
pub mod registry;
mod runner;
mod runner_config;

pub use command::PreparedCommand;
pub use error::{Result, RunnerError};
pub use kind::{Family, RunnerKind};
pub use profile::PermissionProfile;
pub use registry::{ProfileSpec, PromptVia, RunnerRegistry, RunnerSpec};
pub use runner::{
    assert_cli_on_path, for_kind, for_kind_with_registry, ClaudeRunner, CopilotRunner, Runner,
    SpawnContext,
};
pub use runner_config::ConfigRunner;
