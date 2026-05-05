//! `cvg doctor` — diagnose local Convergio setup.

use super::Client;
use super::OutputMode;
use anyhow::{Context, Result};
use convergio_i18n::Bundle;
use serde::Serialize;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Run diagnostics.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    json: bool,
    kill_zombies: bool,
) -> Result<()> {
    if kill_zombies {
        super::doctor_zombies::run().await?;
        return Ok(());
    }
    let report = build_report(client).await;
    match if json { OutputMode::Json } else { output } {
        OutputMode::Human => print_human(bundle, &report),
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Plain => println!("{}", if report.ok { "ok" } else { "fail" }),
    }
    if report.ok {
        Ok(())
    } else {
        anyhow::bail!("doctor found failing checks")
    }
}

#[derive(Serialize)]
struct DoctorReport {
    ok: bool,
    url: String,
    checks: Vec<DoctorCheck>,
}

#[derive(Serialize)]
struct DoctorCheck {
    name: &'static str,
    status: CheckStatus,
    message: String,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    Ok,
    Warn,
    Fail,
}

async fn build_report(client: &Client) -> DoctorReport {
    let mut checks = vec![];
    let home = convergio_home().ok();
    check_config(&mut checks, home.as_deref());
    check_pid(&mut checks, home.as_deref());
    check_binary(&mut checks, "cvg", true, Some(env!("CARGO_PKG_VERSION")));
    check_binary(
        &mut checks,
        "convergio",
        true,
        Some(env!("CARGO_PKG_VERSION")),
    );
    check_binary(&mut checks, "convergio-mcp", false, None);

    let daemon_ok = check_daemon(&mut checks, client).await;
    if daemon_ok {
        check_audit(&mut checks, client).await;
    } else {
        checks.push(DoctorCheck {
            name: "audit",
            status: CheckStatus::Warn,
            message: "skipped because daemon is not reachable".into(),
        });
    }

    DoctorReport {
        ok: checks
            .iter()
            .all(|c| !matches!(c.status, CheckStatus::Fail)),
        url: client.base().to_string(),
        checks,
    }
}

fn check_config(checks: &mut Vec<DoctorCheck>, home: Option<&Path>) {
    let Some(home) = home else {
        checks.push(fail("config_dir", "HOME is not set"));
        return;
    };
    if home.is_dir() {
        checks.push(ok("config_dir", format!("{}", home.display())));
    } else {
        checks.push(fail(
            "config_dir",
            format!("{} missing; run `cvg setup`", home.display()),
        ));
    }

    let config = home.join("config.toml");
    if config.is_file() {
        checks.push(ok("config_file", format!("{}", config.display())));
    } else {
        checks.push(warn(
            "config_file",
            format!("{} missing; run `cvg setup`", config.display()),
        ));
    }
}

fn check_pid(checks: &mut Vec<DoctorCheck>, home: Option<&Path>) {
    let Some(home) = home else {
        return;
    };
    let pid_path = home.join("daemon.pid");
    let Ok(raw) = std::fs::read_to_string(&pid_path) else {
        checks.push(warn("daemon_pid", "no daemon.pid found"));
        return;
    };
    let pid = raw.trim();
    if pid.is_empty() {
        checks.push(warn("daemon_pid", "daemon.pid is empty"));
        return;
    }
    let alive = Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if alive {
        checks.push(ok("daemon_pid", format!("pid {pid} is running")));
    } else {
        checks.push(warn("daemon_pid", format!("stale pid {pid}")));
    }
}

fn check_binary(
    checks: &mut Vec<DoctorCheck>,
    name: &'static str,
    required: bool,
    expected_version: Option<&str>,
) {
    let status = Command::new(name).arg("--version").output();
    match status {
        Ok(out) if out.status.success() => {
            let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if let Some(expected) = expected_version {
                if version.contains(expected) {
                    checks.push(ok(name, version));
                } else {
                    checks.push(fail(
                        name,
                        format!("{version}; expected version {expected}. Run install-local.sh"),
                    ));
                }
            } else {
                checks.push(ok(name, version));
            }
        }
        _ if required => checks.push(fail(name, format!("`{name}` not found in PATH"))),
        _ => checks.push(warn(name, format!("`{name}` not found yet"))),
    }
}

async fn check_daemon(checks: &mut Vec<DoctorCheck>, client: &Client) -> bool {
    let url = format!("{}/v1/health", client.base());
    let resp = reqwest::get(&url).await;
    match resp {
        Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
            Ok(body) if body.get("service").and_then(Value::as_str) == Some("convergio") => {
                let version = body.get("version").and_then(Value::as_str).unwrap_or("?");
                if version == env!("CARGO_PKG_VERSION") {
                    checks.push(ok("daemon", format!("version {version}")));
                    true
                } else {
                    checks.push(fail(
                        "daemon",
                        format!("version {version}; expected {}", env!("CARGO_PKG_VERSION")),
                    ));
                    false
                }
            }
            Ok(_) => {
                checks.push(fail("daemon", "service is not convergio"));
                false
            }
            Err(e) => {
                checks.push(fail("daemon", format!("invalid health JSON: {e}")));
                false
            }
        },
        Ok(resp) => {
            checks.push(fail("daemon", format!("HTTP {}", resp.status())));
            false
        }
        Err(e) => {
            checks.push(fail("daemon", format!("unreachable: {e}")));
            false
        }
    }
}

async fn check_audit(checks: &mut Vec<DoctorCheck>, client: &Client) {
    match client.get::<Value>("/v1/audit/verify").await {
        Ok(report) if report.get("ok").and_then(Value::as_bool) == Some(true) => {
            checks.push(ok("audit", "chain verifies"));
        }
        Ok(report) => checks.push(fail("audit", format!("chain failed: {report}"))),
        Err(e) => checks.push(fail("audit", format!("{e}"))),
    }
}

fn convergio_home() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME is not set")?;
    Ok(PathBuf::from(home).join(".convergio"))
}

fn ok(name: &'static str, message: impl Into<String>) -> DoctorCheck {
    DoctorCheck {
        name,
        status: CheckStatus::Ok,
        message: message.into(),
    }
}

fn warn(name: &'static str, message: impl Into<String>) -> DoctorCheck {
    DoctorCheck {
        name,
        status: CheckStatus::Warn,
        message: message.into(),
    }
}

fn fail(name: &'static str, message: impl Into<String>) -> DoctorCheck {
    DoctorCheck {
        name,
        status: CheckStatus::Fail,
        message: message.into(),
    }
}

fn print_human(bundle: &Bundle, report: &DoctorReport) {
    println!("{}", bundle.t("doctor-header", &[("url", &report.url)]));
    for check in &report.checks {
        let key = match check.status {
            CheckStatus::Ok => "doctor-ok",
            CheckStatus::Warn => "doctor-warn",
            CheckStatus::Fail => "doctor-fail",
        };
        println!(
            "{}",
            bundle.t(key, &[("name", check.name), ("message", &check.message)])
        );
    }
    let summary = if report.ok {
        "doctor-summary-ok"
    } else {
        "doctor-summary-fail"
    };
    println!("{}", bundle.t(summary, &[]));
}
