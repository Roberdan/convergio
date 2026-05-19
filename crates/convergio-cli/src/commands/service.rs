//! `cvg service ...` — install and control the user daemon service.

use anyhow::{bail, Context, Result};
use clap::Subcommand;
use convergio_i18n::Bundle;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use super::service_port::{
    daemon_port, kill_pid, pid_on_port, wait_for_port_bound, wait_for_port_release,
};
use super::service_unit::{launchd_plist, systemd_unit};

pub(super) const LABEL: &str = "com.convergio.v3";
const SERVICE: &str = "convergio.service";

/// User-level service subcommands.
#[derive(Subcommand)]
pub enum ServiceCommand {
    /// Write the user service file.
    Install {
        /// Overwrite an existing service file.
        #[arg(long)]
        force: bool,
    },
    /// Start or reload the user service.
    Start,
    /// Stop the user service.
    Stop,
    /// Show whether the service manager reports it as loaded.
    Status,
    /// Stop and remove the user service file.
    Uninstall,
}

/// Run a service subcommand.
pub async fn run(bundle: &Bundle, cmd: ServiceCommand) -> Result<()> {
    let service = ServiceSpec::current()?;
    match cmd {
        ServiceCommand::Install { force } => {
            service.install(force)?;
            println!(
                "{}",
                bundle.t(
                    "service-installed",
                    &[("path", &service.path.display().to_string())]
                )
            );
        }
        ServiceCommand::Start => {
            service.start()?;
            println!("{}", bundle.t("service-started", &[]));
        }
        ServiceCommand::Stop => {
            service.stop()?;
            println!("{}", bundle.t("service-stopped", &[]));
        }
        ServiceCommand::Status => {
            let key = if service.is_loaded()? {
                "service-status-loaded"
            } else {
                "service-status-not-loaded"
            };
            println!("{}", bundle.t(key, &[]));
        }
        ServiceCommand::Uninstall => {
            service.stop_best_effort();
            if service.path.exists() {
                fs::remove_file(&service.path)
                    .with_context(|| format!("remove {}", service.path.display()))?;
            }
            println!("{}", bundle.t("service-uninstalled", &[]));
        }
    }
    Ok(())
}

enum ServiceKind {
    Launchd,
    Systemd,
}

struct ServiceSpec {
    kind: ServiceKind,
    path: PathBuf,
    content: String,
}

impl ServiceSpec {
    fn current() -> Result<Self> {
        let home = home()?;
        let convergio = resolve_binary("convergio")?;
        if cfg!(target_os = "macos") {
            let path = home
                .join("Library/LaunchAgents")
                .join(format!("{LABEL}.plist"));
            Ok(Self {
                kind: ServiceKind::Launchd,
                path,
                content: launchd_plist(&convergio, &home),
            })
        } else if cfg!(target_os = "linux") {
            let path = home.join(".config/systemd/user").join(SERVICE);
            Ok(Self {
                kind: ServiceKind::Systemd,
                path,
                content: systemd_unit(&convergio),
            })
        } else {
            bail!("user service management is supported on macOS and Linux")
        }
    }

    fn install(&self, force: bool) -> Result<()> {
        if self.path.exists() && !force {
            return Ok(());
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
        }
        fs::write(&self.path, &self.content)
            .with_context(|| format!("write {}", self.path.display()))
    }

    fn start(&self) -> Result<()> {
        self.install(false)?;
        let port = daemon_port();

        // Phase 1: register the service with the manager.
        // `launchctl bootstrap` only *registers* the spec — with the
        // post-2026-05-08 hardened plist (`RunAtLoad=false`,
        // `KeepAlive=false`) it does not actually spawn the process.
        // Pre-fix the CLI returned "Service started." here, the port
        // stayed unbound, and operators only found out via the next
        // failing `curl /v1/health`.
        // See `docs/incidents/2026-05-19-cvg-service-start-orphan.md`.
        let domain = match self.kind {
            ServiceKind::Launchd => {
                let d = format!("gui/{}", uid()?);
                // bootstrap is idempotent only if the service isn't
                // already loaded; ignore "already bootstrapped" errors
                // and rely on the kickstart below.
                let _ = run_cmd("launchctl", &["bootstrap", &d, path_str(&self.path)?]);
                Some(d)
            }
            ServiceKind::Systemd => {
                run_cmd("systemctl", &["--user", "daemon-reload"])?;
                run_cmd("systemctl", &["--user", "enable", "--now", SERVICE])?;
                None
            }
        };

        // Phase 2: if the port is already bound (e.g. the operator
        // ran `convergio start` from a terminal earlier, or a previous
        // bootstrap is still serving), nothing else to do.
        if wait_for_port_bound(port, Duration::from_secs(1)) {
            return Ok(());
        }

        // Phase 3: actively kick the service. For launchd we need
        // `kickstart` because `RunAtLoad=false`; for systemd
        // `--now` already started it.
        if let Some(domain) = domain {
            let _ = run_cmd("launchctl", &["kickstart", &format!("{domain}/{LABEL}")]);
        }

        // Phase 4: verify the daemon actually came up. If not, the
        // operator sees an error instead of the previous Ok lie.
        if wait_for_port_bound(port, Duration::from_secs(5)) {
            return Ok(());
        }
        bail!("daemon port {port} did not bind after service start")
    }

    fn stop(&self) -> Result<()> {
        // Phase 1: ask the service manager. With the post-2026-05-08
        // hardened plist (`KeepAlive=false`, `RunAtLoad=false`),
        // `launchctl bootout` removes the *registration* but does
        // not touch orphan PIDs launched outside launchd. We
        // therefore treat the manager call as best-effort and rely
        // on the port-release check below as the real contract.
        // See `docs/incidents/2026-05-19-cvg-service-stop-orphan.md`.
        let _ = match self.kind {
            ServiceKind::Launchd => run_cmd(
                "launchctl",
                &["bootout", &format!("gui/{}", uid()?), path_str(&self.path)?],
            ),
            ServiceKind::Systemd => run_cmd("systemctl", &["--user", "stop", SERVICE]),
        };

        // Phase 2: verify the daemon port is actually released.
        let port = daemon_port();
        if wait_for_port_release(port, Duration::from_secs(3)) {
            return Ok(());
        }
        if let Some(pid) = pid_on_port(port) {
            eprintln!("warning: daemon PID {pid} survived service manager stop; sending SIGTERM");
            let _ = kill_pid(pid, "-TERM");
            if wait_for_port_release(port, Duration::from_secs(3)) {
                return Ok(());
            }
            eprintln!("warning: PID {pid} still bound after SIGTERM; escalating to SIGKILL");
            let _ = kill_pid(pid, "-KILL");
            if wait_for_port_release(port, Duration::from_secs(2)) {
                return Ok(());
            }
        }
        bail!("daemon port {port} still bound after stop")
    }

    fn stop_best_effort(&self) {
        let _ = self.stop();
    }

    fn is_loaded(&self) -> Result<bool> {
        let ok = match self.kind {
            ServiceKind::Launchd => Command::new("launchctl")
                .args(["print", &format!("gui/{}/{}", uid()?, LABEL)])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status(),
            ServiceKind::Systemd => Command::new("systemctl")
                .args(["--user", "is-active", "--quiet", SERVICE])
                .status(),
        };
        Ok(ok.map(|s| s.success()).unwrap_or(false))
    }
}

fn run_cmd(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program).args(args).status();
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("{program} failed with {status}"),
        Err(e) => Err(e).with_context(|| format!("run {program}")),
    }
}

fn resolve_binary(name: &str) -> Result<PathBuf> {
    let paths = std::env::var_os("PATH").context("PATH is not set")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }
    bail!("`{name}` not found in PATH; run scripts/install-local.sh")
}

fn path_str(path: &Path) -> Result<&str> {
    path.to_str()
        .with_context(|| format!("path is not valid UTF-8: {}", path.display()))
}

fn home() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var("HOME").context("HOME is not set")?,
    ))
}

fn uid() -> Result<String> {
    let out = Command::new("id").arg("-u").output().context("run id -u")?;
    if !out.status.success() {
        bail!("id -u failed");
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
