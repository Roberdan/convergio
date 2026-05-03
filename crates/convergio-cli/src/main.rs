//! `cvg` — Convergio command-line interface.
//!
//! Pure HTTP client for the Convergio daemon. Does **not** import any
//! server crate — the daemon is the source of truth.
//!
//! All user-facing strings flow through [`convergio_i18n::Bundle`].
//! Locale resolution: `--lang` flag → `CONVERGIO_LANG` env →
//! `LANG`/`LC_ALL` env → fallback `en` (P5).

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use convergio_i18n::{detect_locale, Bundle};

mod commands;

#[derive(Parser)]
#[command(name = "cvg", version, about = "Convergio CLI", long_about = None)]
struct Cli {
    /// Daemon base URL.
    #[arg(
        long,
        global = true,
        env = "CONVERGIO_URL",
        default_value = "http://127.0.0.1:8420"
    )]
    url: String,

    /// User interface language. Falls back to CONVERGIO_LANG / LANG / en.
    #[arg(long, global = true, value_name = "LOCALE")]
    lang: Option<String>,

    /// Output format for commands that support multiple views.
    #[arg(long, global = true, value_enum, default_value_t = commands::OutputMode::Human)]
    output: commands::OutputMode,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Probe the daemon.
    Health,
    /// Initialize local configuration.
    Setup {
        #[command(subcommand)]
        sub: Option<commands::setup::SetupCommand>,
    },
    /// Diagnose local configuration and daemon health.
    Doctor {
        /// Print machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Show active plans and recently completed work.
    Status {
        /// Number of completed plans/tasks to show.
        #[arg(long, default_value_t = 10)]
        completed_limit: i64,
        /// Filter to a single project (e.g. `--project convergio-local`).
        #[arg(long)]
        project: Option<String>,
        /// Include `cvg demo` and live-test artefact plans
        /// (hidden by default to keep the human view legible).
        #[arg(long)]
        all: bool,
        /// Show a per-wave breakdown under each plan.
        #[arg(long)]
        show_waves: bool,
        /// Filter `next` tasks to those owned by the caller.
        /// Identity comes from `CONVERGIO_AGENT_ID` (stop-gap until F46/F47).
        #[arg(long)]
        mine: bool,
    },
    /// Plan operations.
    Plan {
        #[command(subcommand)]
        sub: commands::plan::PlanCommand,
    },
    /// Task operations.
    Task {
        #[command(subcommand)]
        sub: commands::task::TaskCommand,
    },
    /// Evidence operations.
    Evidence {
        #[command(subcommand)]
        sub: commands::evidence::EvidenceCommand,
    },
    /// Audit log operations.
    Audit {
        #[command(subcommand)]
        sub: commands::audit::AuditCommand,
    },
    /// Inspect the durable agent registry (live who-is-on-what).
    Agent {
        #[command(subcommand)]
        sub: commands::agent::AgentCommand,
    },
    /// CRDT diagnostics.
    Crdt {
        #[command(subcommand)]
        sub: commands::crdt::CrdtCommand,
    },
    /// Local capability registry diagnostics.
    Capability {
        #[command(subcommand)]
        sub: commands::capability::CapabilityCommand,
    },
    /// Local cross-document coherence checks (ADR frontmatter, workspace).
    Coherence {
        #[command(subcommand)]
        sub: commands::coherence::CoherenceCommand,
    },
    /// Auto-regenerate derived markdown sections (ADR-0015).
    Docs {
        #[command(subcommand)]
        sub: commands::docs::DocsCommand,
    },
    /// Tier-3 code graph (build, stats; ADR-0014).
    Graph {
        #[command(subcommand)]
        sub: commands::graph::GraphCommand,
    },
    /// Workspace coordination diagnostics.
    Workspace {
        #[command(subcommand)]
        sub: commands::workspace::WorkspaceCommand,
    },
    /// MCP bridge diagnostics.
    Mcp {
        #[command(subcommand)]
        sub: commands::mcp::McpCommand,
    },
    /// Local PR queue dashboard (read-only).
    Pr {
        #[command(subcommand)]
        sub: commands::pr::PrCommand,
    },
    /// User-level daemon service management.
    Service {
        #[command(subcommand)]
        sub: commands::service::ServiceCommand,
    },
    /// Cold-start brief from the daemon (replaces handoff markdown).
    Session {
        #[command(subcommand)]
        sub: commands::session::SessionCommand,
    },
    /// Solve a mission into a plan (Layer 4 planner).
    Solve {
        /// Mission text — newline-separated tasks.
        mission: String,
    },
    /// Run one executor tick (dispatches pending tasks).
    Dispatch,
    /// Run Thor on a plan. Without `--wave` the verdict is
    /// plan-strict (every task must be submitted/done). With
    /// `--wave N` the verdict is restricted to wave N — tasks in
    /// other waves are ignored. T3.06 enables progressive
    /// promotion on long-running backlog plans.
    Validate {
        /// Plan id.
        plan_id: String,
        /// Optional wave number (T3.06). When set, validation
        /// considers only tasks in this wave.
        #[arg(long)]
        wave: Option<i64>,
    },
    /// Print the Convergio brand lockup, claim, and version.
    /// Plays the boot animation on a TTY; static when piped or
    /// when `NO_COLOR` / `CONVERGIO_THEME=mono` is set.
    About {
        /// Force the boot animation even if the theme would skip it.
        #[arg(long)]
        animate: bool,
    },
    /// Run a guided local demo.
    Demo,
    /// Open the read-only TUI dashboard (cvg dash, ADR-0029).
    /// 4-pane htop-style: plans, active tasks, agents, PRs.
    Dash {
        /// Refresh interval in seconds (clamped to [1, 300]).
        #[arg(long, env = "CONVERGIO_DASH_TICK_SECS", default_value_t = 5)]
        tick_secs: u64,
    },
    /// Rebuild and restart the local Convergio daemon (closes F50).
    Update {
        /// Skip rebuild when daemon already matches workspace version.
        #[arg(long)]
        if_needed: bool,
        /// Rebuild and sync binaries but do not restart the daemon.
        #[arg(long)]
        skip_restart: bool,
    },
    /// Inspect (and optionally publish to) the plan-scoped agent
    /// message bus.
    Bus {
        #[command(subcommand)]
        sub: commands::bus::BusCommand,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let locale = detect_locale(cli.lang.as_deref());
    let bundle = Bundle::new(locale).context("load CLI Fluent bundle")?;
    let client = commands::Client::new(cli.url);
    match cli.command {
        Command::Health => commands::health::run(&client, &bundle, cli.output).await,
        Command::Setup { sub } => commands::setup::run(&bundle, sub).await,
        Command::Doctor { json } => commands::doctor::run(&client, &bundle, cli.output, json).await,
        Command::Status {
            completed_limit,
            project,
            all,
            show_waves,
            mine,
        } => {
            commands::status::run(
                &client,
                &bundle,
                cli.output,
                completed_limit,
                project,
                all,
                show_waves,
                mine,
            )
            .await
        }
        Command::Plan { sub } => commands::plan::run(&client, &bundle, cli.output, sub).await,
        Command::Task { sub } => commands::task::run(&client, cli.output, sub).await,
        Command::Evidence { sub } => commands::evidence::run(&client, sub).await,
        Command::Audit { sub } => commands::audit::run(&client, sub).await,
        Command::Agent { sub } => commands::agent::run(&client, &bundle, cli.output, sub).await,
        Command::Crdt { sub } => commands::crdt::run(&client, &bundle, cli.output, sub).await,
        Command::Capability { sub } => {
            commands::capability::run(&client, &bundle, cli.output, sub).await
        }
        Command::Coherence { sub } => commands::coherence::run(cli.output, sub).await,
        Command::Docs { sub } => commands::docs::run(cli.output, sub).await,
        Command::Graph { sub } => commands::graph::run(&client, cli.output, sub).await,
        Command::Workspace { sub } => {
            commands::workspace::run(&client, &bundle, cli.output, sub).await
        }
        Command::Mcp { sub } => commands::mcp::run(&bundle, sub).await,
        Command::Pr { sub } => commands::pr::run(&client, &bundle, cli.output, sub).await,
        Command::Service { sub } => commands::service::run(&bundle, sub).await,
        Command::Session { sub } => commands::session::run(&client, &bundle, cli.output, sub).await,
        Command::Solve { mission } => commands::solve::run(&client, &mission).await,
        Command::Dispatch => commands::dispatch::run(&client).await,
        Command::Validate { plan_id, wave } => {
            commands::validate::run(&client, &plan_id, wave).await
        }
        Command::About { animate } => commands::about::run(&bundle, animate),
        Command::Demo => commands::demo::run(&client).await,
        Command::Dash { tick_secs } => commands::dash::run(client.base(), tick_secs).await,
        Command::Update {
            if_needed,
            skip_restart,
        } => commands::update::run(&client, &bundle, cli.output, if_needed, skip_restart).await,
        Command::Bus { sub } => commands::bus::run(&client, cli.output, sub).await,
    }
}
