//! `cvg` — Convergio CLI (pure HTTP client). i18n via `Bundle`.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use convergio_i18n::{detect_locale, Bundle};

mod commands;
mod dispatch;
mod drift;
mod http;

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
pub(crate) enum Command {
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
        /// Identify and offer to kill stale `target/.../deps/e2e_*`
        /// processes (interactive). See ADR-0044.
        #[arg(long)]
        kill_zombies: bool,
    },
    /// Show active plans and recently completed work.
    Status {
        /// Number of completed plans/tasks to show.
        #[arg(long, default_value_t = 10)]
        completed_limit: i64,
        /// Filter to a single project (e.g. `--project convergio-local`).
        #[arg(long)]
        project: Option<String>,
        /// Include `cvg demo` and live-test artefact plans (hidden by default).
        #[arg(long)]
        all: bool,
        /// Show a per-wave breakdown under each plan.
        #[arg(long)]
        show_waves: bool,
        /// Filter `next` tasks to caller (id from `CONVERGIO_AGENT_ID` env).
        #[arg(long)]
        mine: bool,
        /// Show live agents from the registry instead of plans.
        #[arg(long)]
        agents: bool,
    },
    /// Plan operations.
    Plan {
        #[command(subcommand)]
        sub: commands::plan::PlanCommand,
    },
    /// Parametric plan templates (list / show / render; W6, ADR-0056).
    #[command(name = "plan-templates")]
    PlanTemplates {
        #[command(subcommand)]
        sub: commands::plan_templates::PlanTemplatesCommand,
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
    /// Discoverable action type registry (read-only).
    Actions {
        #[command(subcommand)]
        sub: commands::actions::ActionsCommand,
    },
    /// Discoverable gate precondition catalog (read-only).
    Gates {
        #[command(subcommand)]
        sub: commands::gates::GatesCommand,
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
    /// Tier-3 semantic embeddings (build, warm, for-task; ADR-0038).
    Embed {
        #[command(subcommand)]
        sub: commands::embed::EmbedCommand,
    },
    /// Fleet repo management (add, ls, enable, disable; ADR-0038 F2-6).
    Fleet {
        #[command(subcommand)]
        sub: commands::fleet::FleetCommand,
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
    /// Public transparency artefact generators.
    Public {
        #[command(subcommand)]
        sub: commands::public::PublicCommand,
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
    /// Run one executor tick. Use --executor none/--no-dispatch for tracker-only mode.
    Dispatch {
        /// Do not spawn workers; return tracker-only dispatch status.
        #[arg(long)]
        no_dispatch: bool,
        /// Executor mode. `none` is tracker-only.
        #[arg(long, value_enum, default_value_t = commands::dispatch::ExecutorMode::Default)]
        executor: commands::dispatch::ExecutorMode,
    },
    /// Run Thor on a plan, or `--self-test` for the H11 fixture run.
    Validate {
        /// Plan id (omit when `--self-test` is set).
        plan_id: Option<String>,
        /// Optional wave number — when set, only tasks in this wave (T3.06).
        #[arg(long)]
        wave: Option<i64>,
        /// Exercise Thor against a fresh fixture plan + task (P2-7).
        #[arg(long, default_value_t = false)]
        self_test: bool,
    },
    /// Print the Convergio brand lockup, claim, and version.
    About {
        /// Force the boot animation even if the theme would skip it.
        #[arg(long)]
        animate: bool,
    },
    /// Stream daemon audit events brand-coloured (Ctrl-C exits).
    Monitor {
        /// Poll interval in seconds (clamped to `[1, 60]`).
        #[arg(long, env = "CONVERGIO_MONITOR_TICK_SECS", default_value_t = 1)]
        tick_secs: u64,
    },
    /// Run a guided local demo.
    Demo,
    /// Open the read-only TUI dashboard (cvg dash, ADR-0029).
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
        /// After install, print the CHANGELOG slice for the new version.
        #[arg(long)]
        changelog: bool,
    },
    /// Inspect (and optionally publish to) the plan-scoped agent message bus.
    Bus {
        #[command(subcommand)]
        sub: commands::bus::BusCommand,
    },
    /// Ontology Runtime Core registry (list, describe, export; ADR-0053).
    Ontology {
        #[command(subcommand)]
        sub: commands::ontology::OntologyCommand,
    },
    /// Purpose registry: declare and list immutable processing purposes (ADR-0054).
    Purpose {
        #[command(subcommand)]
        sub: commands::purpose::PurposeCommand,
    },
    /// One-shot peer + bus + plan snapshot for a fresh agent session.
    Discover {
        /// Lookback window for active peers / bus topics. Default `30m`.
        #[arg(long, default_value = "30m")]
        since: String,
        /// Caller agent id (else `CONVERGIO_AGENT_ID` env, else `claude-code-${USER}`).
        #[arg(long, env = "CONVERGIO_AGENT_ID")]
        agent_id: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let locale = detect_locale(cli.lang.as_deref());
    let bundle = Bundle::new(locale).context("load CLI Fluent bundle")?;
    if !matches!(
        cli.command,
        Command::Setup { .. } | Command::Service { .. } | Command::About { .. }
    ) {
        drift::check_and_warn(&bundle, &cli.url, env!("CARGO_PKG_VERSION")).await;
    }
    let client = commands::Client::new(cli.url);
    dispatch::run(client, bundle, cli.output, cli.command).await
}
