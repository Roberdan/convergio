//! Step driver for `cvg update` (F50). Sequence: probe -> rebuild
//! three crates -> sync `~/.cargo/bin` to `~/.local/bin` (F44) ->
//! stop + restart daemon via the platform service manager (launchd /
//! systemd) so the new process survives beyond the calling git hook's
//! process group -> re-probe + verify audit chain.
//! `--if-needed` short-circuits when versions already match.

use super::service::restart_via_service_manager;
use super::Client;
use super::OutputMode;
use anyhow::{Context, Result};
use convergio_i18n::Bundle;
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

/// Caller-visible flags translated from clap.
#[derive(Clone, Copy, Debug)]
pub struct UpdateOptions {
    /// Skip the rebuild when daemon already matches workspace.
    pub if_needed: bool,
    /// Rebuild and sync, but do not restart the daemon.
    pub skip_restart: bool,
}

/// What `cvg update` produced. Rendered by the caller.
#[derive(Clone, Debug)]
pub struct UpdateOutcome {
    /// True when at least one binary was rebuilt.
    pub rebuilt: bool,
    /// True when the daemon was restarted.
    pub restarted: bool,
    /// Daemon version reported before the update (or "unknown").
    pub prior_version: String,
    /// Daemon version reported after the update (or workspace version
    /// when daemon is unreachable post-restart).
    pub new_version: String,
    /// Audit chain verification result post-restart.
    pub audit_chain_ok: bool,
    /// True when `--if-needed` short-circuited the rebuild.
    pub skipped_no_update_needed: bool,
}

/// Drive the update steps and return the outcome record.
pub async fn run_update(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    opts: UpdateOptions,
) -> Result<UpdateOutcome> {
    let workspace_version = env!("CARGO_PKG_VERSION").to_string();
    let prior_version = probe_daemon_version(client)
        .await
        .unwrap_or_else(|_| "unknown".into());

    if opts.if_needed && prior_version == workspace_version {
        return Ok(UpdateOutcome {
            rebuilt: false,
            restarted: false,
            prior_version: prior_version.clone(),
            new_version: prior_version,
            audit_chain_ok: probe_audit(client).await.unwrap_or(false),
            skipped_no_update_needed: true,
        });
    }

    if matches!(output, OutputMode::Human) {
        println!("{}", bundle.t("update-rebuild-header", &[]));
    }
    rebuild_all(bundle, output)?;

    if matches!(output, OutputMode::Human) {
        println!("{}", bundle.t("update-sync-header", &[]));
    }
    sync_shadowed_binaries(bundle)?;

    let restarted = if opts.skip_restart {
        if matches!(output, OutputMode::Human) {
            println!("{}", bundle.t("update-restart-skipped", &[]));
        }
        false
    } else {
        if matches!(output, OutputMode::Human) {
            println!("{}", bundle.t("update-restart-header", &[]));
        }
        restart_daemon(&workspace_version)?;
        true
    };

    if matches!(output, OutputMode::Human) {
        println!("{}", bundle.t("update-verify-header", &[]));
    }
    let new_version = probe_daemon_version(client)
        .await
        .unwrap_or_else(|_| workspace_version.clone());
    let audit_chain_ok = probe_audit(client).await.unwrap_or(false);

    Ok(UpdateOutcome {
        rebuilt: true,
        restarted,
        prior_version,
        new_version,
        audit_chain_ok,
        skipped_no_update_needed: false,
    })
}

async fn probe_daemon_version(client: &Client) -> Result<String> {
    let body: Value = client.get("/v1/health").await?;
    Ok(body
        .get("running_version")
        .and_then(Value::as_str)
        .or_else(|| body.get("version").and_then(Value::as_str))
        .unwrap_or("unknown")
        .to_string())
}

async fn probe_audit(client: &Client) -> Result<bool> {
    let body: Value = client.get("/v1/audit/verify").await?;
    Ok(body.get("ok").and_then(Value::as_bool).unwrap_or(false))
}

fn cargo_bin() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".cargo").join("bin"))
}

fn local_bin() -> Option<PathBuf> {
    home_dir().map(|h| h.join(".local").join("bin"))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn rebuild_all(bundle: &Bundle, output: OutputMode) -> Result<()> {
    let workspace_root = super::update_repo_root::resolve().context("locate workspace root")?;
    for crate_name in ["convergio-server", "convergio-cli", "convergio-mcp"] {
        if matches!(output, OutputMode::Human) {
            println!(
                "  {}",
                bundle.t("update-rebuild-step", &[("crate", crate_name)])
            );
        }
        run_step(
            "cargo install",
            Command::new("cargo")
                .arg("install")
                .arg("--path")
                .arg(workspace_root.join("crates").join(crate_name))
                .arg("--force")
                .arg("--locked"),
        )?;
    }
    Ok(())
}

fn sync_shadowed_binaries(bundle: &Bundle) -> Result<()> {
    let cargo_bin = cargo_bin().context("HOME is not set")?;
    let local_bin = local_bin().context("HOME is not set")?;
    if !local_bin.is_dir() {
        // F44: ~/.local/bin may not exist on a fresh box. Don't error,
        // just skip — `cvg doctor` already covers binary discovery.
        return Ok(());
    }
    for bin in ["convergio", "cvg", "convergio-mcp"] {
        let src = cargo_bin.join(bin);
        let dst = local_bin.join(bin);
        if src.is_file() {
            // F44 contract: always overwrite, regardless of which copy
            // PATH currently resolves to.
            if let Err(e) = std::fs::copy(&src, &dst) {
                let src = src.display().to_string();
                let dst = dst.display().to_string();
                let reason = e.to_string();
                eprintln!(
                    "{}",
                    bundle.t(
                        "update-sync-copy-warning",
                        &[("src", &src), ("dst", &dst), ("reason", &reason)]
                    )
                );
            }
        }
    }
    Ok(())
}

fn restart_daemon(_workspace_version: &str) -> Result<()> {
    // Restart via the platform service manager (launchd / systemd) so
    // the daemon process is adopted by the init system and survives
    // beyond the calling git hook's process group. Previously this used
    // `pkill + spawn` which created a direct child that died with the
    // hook; see the 2026-06-14 `cvg dash` incident.
    //
    // PATH (F45) and CONVERGIO_REPO_PATH are already set in the service
    // unit file; CONVERGIO_EXPECTED_VERSION is omitted because the
    // freshly-installed binary always matches the workspace version, so
    // the drift flag would never fire in this flow.
    restart_via_service_manager()
}

fn run_step(label: &str, cmd: &mut Command) -> Result<()> {
    let status = cmd.status().with_context(|| format!("spawn {label}"))?;
    if !status.success() {
        anyhow::bail!("{label} failed with status {}", status.code().unwrap_or(-1));
    }
    Ok(())
}
