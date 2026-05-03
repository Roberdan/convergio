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
use convergio_durability::{init as init_durability, Durability};
use convergio_executor::{spawn_loop as executor_spawn_loop, Executor, SpawnTemplate};
use convergio_lifecycle::watcher::{self, WatcherConfig};
use convergio_lifecycle::Supervisor;
use convergio_server::{router, AppState};
use std::io::{self, IsTerminal};
use std::net::SocketAddr;
use std::sync::Arc;
use tracing_subscriber::{fmt, EnvFilter};

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
    write_pid_file()?;

    play_boot_banner();

    tracing::info!(%db_url, %bind, "starting convergio daemon");

    let pool = Pool::connect(&db_url).await?;
    init_durability(&pool).await?;
    convergio_bus::init(&pool).await?;
    convergio_lifecycle::init(&pool).await?;
    let graph = Arc::new(convergio_graph::Store::new(pool.clone()));
    graph.migrate().await?;

    let durability = Arc::new(Durability::new(pool.clone()));
    let bus = Arc::new(Bus::new(pool.clone()));
    let supervisor = Arc::new(Supervisor::new(pool));

    let reaper_config = ReaperConfig {
        timeout: Duration::seconds(parse_env_i64("CONVERGIO_REAPER_TIMEOUT_SECS", 300)),
        tick_interval: Duration::seconds(parse_env_i64("CONVERGIO_REAPER_TICK_SECS", 60)),
    };
    let _reaper = reaper::spawn(durability.clone(), reaper_config);

    let watcher_config = WatcherConfig {
        tick_interval: Duration::seconds(parse_env_i64("CONVERGIO_WATCHER_TICK_SECS", 30)),
    };
    let _watcher = watcher::spawn((*supervisor).clone(), watcher_config);

    let executor = Arc::new(Executor::new(
        (*durability).clone(),
        (*supervisor).clone(),
        SpawnTemplate::default(),
    ));
    let executor_tick = Duration::seconds(parse_env_i64("CONVERGIO_EXECUTOR_TICK_SECS", 30));
    let _executor_loop = executor_spawn_loop(executor, executor_tick);

    let state = AppState {
        durability: durability.clone(),
        bus: bus.clone(),
        supervisor: supervisor.clone(),
        graph: graph.clone(),
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

fn write_pid_file() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let dir = std::path::Path::new(&home).join(".convergio");
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("daemon.pid"), std::process::id().to_string())?;
    Ok(())
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
