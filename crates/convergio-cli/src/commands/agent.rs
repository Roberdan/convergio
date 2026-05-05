//! `cvg agent ...` — surface the durable agent registry as a CLI
//! query.
//!
//! Closes the F46 half-wired bit (F55 in friction log): the daemon
//! sync of `agents.current_task_id` was already in main, but the
//! only way to observe it was direct sqlite SELECT. This command
//! turns the live state query into a first-class human/JSON/plain
//! surface.
//!
//! P0.4 (this iteration): `list` now defaults to active agents
//! (filter by heartbeat threshold, hide terminated/retired), pulls
//! the enriched `summaries` payload (task title, branch, lease
//! count, last audit), `show` switches to the rich
//! `details` view, and `retire-stale` is a new bulk cleanup.

use super::agent_list::{self, ColumnProfile, ListArgs};
use super::agent_retire::{self, RetireArgs};
use super::agent_retire_one;
use super::agent_show;
use super::agent_spawn;
use super::{Client, OutputMode};
use anyhow::Result;
use clap::Subcommand;
use convergio_i18n::Bundle;
use std::path::PathBuf;

/// Agent registry subcommands.
#[derive(Subcommand)]
pub enum AgentCommand {
    /// List registered agents (defaults to active — last
    /// heartbeat within `--threshold-min`).
    List {
        /// Show terminated/retired agents too.
        #[arg(long)]
        all: bool,
        /// Heartbeat freshness threshold, minutes (default 30).
        #[arg(long, default_value_t = 30)]
        threshold_min: i64,
        /// Column profile.
        #[arg(long, value_enum, default_value_t = ColumnProfile::Default)]
        columns: ColumnProfile,
    },
    /// Show a single agent record by id (rich, multi-section view).
    Show {
        /// Agent id (e.g. `claude-code-roberdan`).
        id: String,
    },
    /// Retire a single agent by id (idempotent: hits
    /// `POST /v1/agent-registry/agents/:id/retire`).
    Retire {
        /// Agent id to retire (e.g. `subagent-p1-5`).
        id: String,
    },
    /// Bulk-retire agents whose heartbeat is older than the
    /// threshold. Dry-run by default.
    RetireStale {
        /// Heartbeat staleness threshold, minutes (default 60).
        #[arg(long, default_value_t = 60)]
        threshold_min: i64,
        /// Actually retire matched agents (default: dry-run).
        #[arg(long)]
        apply: bool,
    },
    /// Spawn a vendor-CLI agent against a single task (ADR-0032).
    ///
    /// Loads the task + plan + (optional) graph context-pack from
    /// the daemon, hands them to the right runner, and either
    /// prints the prepared command (`--dry-run`) or executes it
    /// inline. Auth, billing and rate-limiting all live in the
    /// vendor CLI — Convergio never sees an API key.
    Spawn {
        /// Task id to work on.
        #[arg(long)]
        task: String,
        /// Runner kind in the wire format `<vendor>:<model>`
        /// (e.g. `claude:sonnet`, `claude:opus`, `copilot:gpt-5.2`,
        /// `copilot:claude-opus`). Default: `claude:sonnet`.
        #[arg(long, default_value = "claude:sonnet")]
        runner: String,
        /// Stable agent identity. Default: `<vendor>-<model>-<task7>`.
        #[arg(long)]
        agent_id: Option<String>,
        /// Working directory for the spawned CLI. Default: the
        /// current shell cwd.
        #[arg(long)]
        cwd: Option<PathBuf>,
        /// Per-session budget cap in USD (Claude only — forwarded
        /// to `claude --max-budget-usd`).
        #[arg(long)]
        max_budget_usd: Option<f32>,
        /// Permission profile (ADR-0033). `standard` whitelists
        /// build / edit / `cvg` / `gh` and denies `rm`/`sudo`/
        /// push-to-main. `read_only` blocks edits + bash. `sandbox`
        /// keeps the legacy `--dangerously-skip-permissions` /
        /// `--allow-all` for sealed environments.
        #[arg(long, default_value = "standard")]
        profile: String,
        /// Print the argv + prompt without spawning the CLI.
        #[arg(long)]
        dry_run: bool,
    },
}

/// Dispatch.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    cmd: AgentCommand,
) -> Result<()> {
    match cmd {
        AgentCommand::List {
            all,
            threshold_min,
            columns,
        } => {
            agent_list::run(
                client,
                bundle,
                output,
                ListArgs {
                    all,
                    threshold_min,
                    columns,
                },
            )
            .await
        }
        AgentCommand::Show { id } => agent_show::run(client, bundle, output, &id).await,
        AgentCommand::Retire { id } => agent_retire_one::run(client, bundle, output, &id).await,
        AgentCommand::RetireStale {
            threshold_min,
            apply,
        } => {
            agent_retire::run(
                client,
                bundle,
                output,
                RetireArgs {
                    threshold_min,
                    apply,
                },
            )
            .await
        }
        AgentCommand::Spawn {
            task,
            runner,
            agent_id,
            cwd,
            max_budget_usd,
            profile,
            dry_run,
        } => {
            agent_spawn::run(
                client,
                output,
                agent_spawn::SpawnArgs {
                    task_id: task,
                    runner,
                    agent_id,
                    cwd,
                    max_budget_usd,
                    profile,
                    dry_run,
                },
            )
            .await
        }
    }
}
