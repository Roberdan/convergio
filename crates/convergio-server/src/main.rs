//! Convergio daemon entry point.
//!
//! Boots the local HTTP server, runs SQLite migrations, and spawns the
//! background reaper, watcher, and executor loops.

use chrono::Duration;
use clap::{Parser, Subcommand};
use convergio_brand::{boot, theme::Theme};
use convergio_bus::Bus;
use convergio_db::Pool;
use convergio_durability::reaper::{self, ReaperConfig};
use convergio_durability::telemetry_collector::{self, CollectorConfig};
use convergio_durability::{init as init_durability, Durability};
use convergio_executor::{
    spawn_loop as executor_spawn_loop, Executor, RunnerDefaults, SpawnTemplate,
};
use convergio_lifecycle::watcher::{self, WatcherConfig};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use std::io::{self, IsTerminal};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{fmt, EnvFilter};

mod embedder_setup;
mod pid_lock;
use embedder_setup::make_embedder;

#[derive(Parser)]
#[command(name = "convergio", version, about = "Local Convergio daemon", long_about = None)]
struct Cli {
    /// SQLite database URL.
    #[arg(long, global = true, value_name = "URL", env = "CONVERGIO_DB")]
    db: Option<String>,

    /// TCP bind address. Keep the default localhost bind for local-only use.
    #[arg(long, global = true, value_name = "ADDR", env = "CONVERGIO_BIND")]
    bind: Option<SocketAddr>,

    /// Allow binding outside localhost. This exposes the local spawn API.
    #[arg(
        long,
        global = true,
        env = "CONVERGIO_ALLOW_NON_LOCAL_BIND",
        default_value_t = false
    )]
    allow_non_local_bind: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the local daemon.
    Start,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Cli {
        db,
        bind,
        allow_non_local_bind,
        command,
    } = Cli::parse();

    fmt()
        .with_env_filter(
            EnvFilter::try_from_env("CONVERGIO_LOG").unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    match command.unwrap_or(Command::Start) {
        Command::Start => start(db, bind, allow_non_local_bind).await?,
    }
    Ok(())
}

async fn start(
    db: Option<String>,
    bind: Option<SocketAddr>,
    allow_non_local_bind: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let db_url = db.unwrap_or_else(default_sqlite_url);
    let bind = bind.unwrap_or(SocketAddr::from(([127, 0, 0, 1], 8420)));
    ensure_local_bind(bind, allow_non_local_bind)?;
    pid_lock::claim()?;

    play_boot_banner();

    tracing::info!(%db_url, %bind, "starting convergio daemon");

    let pool = Pool::connect(&db_url).await?;
    init_durability(&pool).await?;
    convergio_bus::init(&pool).await?;
    convergio_lifecycle::init(&pool).await?;
    let graph = Arc::new(convergio_graph::Store::new(pool.clone()));
    graph.migrate().await?;
    convergio_embed::init(&pool).await?;
    let embed = Arc::new(convergio_embed::EmbedStore::new(pool.clone()));
    let embedder = make_embedder();
    convergio_fleet::init(&pool).await?;
    let fleet = Arc::new(convergio_fleet::FleetStore::new(pool.clone()));
    let fleet_plans = Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone()));

    let durability = Arc::new(Durability::new(pool.clone()));
    let bus = Arc::new(Bus::new(pool.clone()));
    let supervisor = Arc::new(Supervisor::new_with_bus(pool, (*bus).clone()));

    let reaper_config = ReaperConfig {
        timeout: Duration::seconds(parse_env_i64("CONVERGIO_REAPER_TIMEOUT_SECS", 300)),
        tick_interval: Duration::seconds(parse_env_i64("CONVERGIO_REAPER_TICK_SECS", 60)),
        agent_reaper_enabled: parse_env_bool("CONVERGIO_AGENT_REAPER_ENABLED", true),
        agent_threshold: Duration::seconds(parse_env_i64(
            "CONVERGIO_AGENT_REAPER_THRESHOLD_SECS",
            3600,
        )),
    };
    let _reaper = reaper::spawn(durability.clone(), reaper_config);

    let watcher_config = WatcherConfig {
        tick_interval: Duration::seconds(parse_env_i64("CONVERGIO_WATCHER_TICK_SECS", 30)),
    };
    let _watcher = watcher::spawn((*supervisor).clone(), watcher_config);

    let runner_defaults = RunnerDefaults::from_env();
    let repo_path = std::env::var_os("CONVERGIO_REPO_PATH").map(std::path::PathBuf::from);
    tracing::info!(
        runner_kind = %runner_defaults.kind,
        runner_profile = ?runner_defaults.profile,
        repo_path = ?repo_path,
        "executor runner defaults"
    );
    if repo_path.is_none() {
        tracing::warn!(
            "CONVERGIO_REPO_PATH unset — runner-based dispatch will refuse to spawn (each task \
             requires a pre-created git worktree under <repo>/.claude/worktrees). The legacy \
             /bin/echo template still works."
        );
    }
    let mut exec = Executor::new(
        (*durability).clone(),
        (*supervisor).clone(),
        SpawnTemplate::default(),
    )
    .with_defaults(runner_defaults);
    if let Some(p) = repo_path {
        exec = exec.with_repo_path(p);
    }
    let executor = Arc::new(exec);
    let executor_tick = Duration::seconds(parse_env_i64("CONVERGIO_EXECUTOR_TICK_SECS", 30));
    let _executor_loop = executor_spawn_loop(executor, executor_tick);

    let collector_config = CollectorConfig {
        tick_interval: Duration::seconds(parse_env_i64("CONVERGIO_TELEMETRY_TICK_SECS", 60)),
    };
    let _telemetry_collector = telemetry_collector::spawn(durability.clone(), collector_config);

    let state = AppState {
        durability: durability.clone(),
        bus: bus.clone(),
        supervisor: supervisor.clone(),
        graph: graph.clone(),
        embed: embed.clone(),
        embedder: embedder.clone(),
        fleet: fleet.clone(),
        fleet_plans: fleet_plans.clone(),
        audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
    };
    let app = router(state);

    let listener = tokio::net::TcpListener::bind(bind).await?;
    tracing::info!(%bind, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}

fn ensure_local_bind(
    bind: SocketAddr,
    allow_non_local_bind: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if bind.ip().is_loopback() || allow_non_local_bind {
        return Ok(());
    }
    Err(format!(
        "refusing to bind {bind}; Convergio is local-first and /v1/agents/spawn can execute local processes. Use --allow-non-local-bind only if you accept that risk."
    )
    .into())
}

fn default_sqlite_url() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    format!("sqlite://{home}/.convergio/v3/state.db?mode=rwc")
}

fn play_boot_banner() {
    let stdout = io::stdout();
    if !stdout.is_terminal() {
        return;
    }
    let theme = Theme::resolve(true);
    let mut handle = stdout.lock();
    let mut sleeper = boot::RealSleeper;
    let _ = boot::play(&mut handle, &mut sleeper, theme);
}

fn parse_env_i64(key: &str, default: i64) -> i64 {
    std::env::var(key)
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(default)
}

fn parse_env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key).as_deref() {
        Ok("0" | "false" | "no") => false,
        Ok("1" | "true" | "yes") => true,
        Ok(_) | Err(_) => default,
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_local_bind;
    use std::net::SocketAddr;

    #[test]
    fn local_bind_is_allowed_by_default() {
        let bind: SocketAddr = "127.0.0.1:8420".parse().expect("valid address");
        assert!(ensure_local_bind(bind, false).is_ok());
    }

    #[test]
    fn non_local_bind_requires_explicit_opt_in() {
        let bind: SocketAddr = "0.0.0.0:8420".parse().expect("valid address");
        assert!(ensure_local_bind(bind, false).is_err());
        assert!(ensure_local_bind(bind, true).is_ok());
    }
}
