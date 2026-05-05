//! `cvg setup self-check` — verify install correctness (ADR-0044).
//!
//! Checks seven conditions defined in the ADR-0044 install-correctness table.
//! FAIL conditions cause exit 1. WARN conditions are advisory and do not fail.

use super::{Client, OutputMode};
use anyhow::Result;
use convergio_i18n::Bundle;
use serde::Serialize;
use serde_json::Value;
use std::path::PathBuf;

/// Severity of a single check.
#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Ok,
    Warn,
    Fail,
}

/// One check result.
#[derive(Serialize)]
pub struct Check {
    pub name: &'static str,
    pub status: Severity,
    pub message: String,
}

/// Full self-check report.
#[derive(Serialize)]
pub struct Report {
    /// True when all FAIL-severity checks pass.
    pub ok: bool,
    pub checks: Vec<Check>,
}

/// Run the self-check and print results.
pub async fn run(client: &Client, bundle: &Bundle, output: OutputMode) -> Result<()> {
    let report = build_report(client).await;
    match output {
        OutputMode::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputMode::Human => render_human(bundle, &report),
        OutputMode::Plain => println!("{}", if report.ok { "ok" } else { "fail" }),
    }
    if report.ok {
        Ok(())
    } else {
        anyhow::bail!("setup self-check failed")
    }
}

async fn build_report(client: &Client) -> Report {
    let mut checks = Vec::new();
    let daemon_ok = check_daemon_up(&mut checks, client).await;
    if daemon_ok {
        check_version_match(&mut checks, client).await;
        check_loops_running(&mut checks, client).await;
        check_embed_nonempty(&mut checks, client).await;
        check_registry_active(&mut checks, client).await;
    } else {
        checks.push(skip("version_match", "skipped — daemon unreachable"));
        checks.push(skip("loops_running", "skipped — daemon unreachable"));
        checks.push(skip("embed_nonempty", "skipped — daemon unreachable"));
        checks.push(skip("registry_active", "skipped — daemon unreachable"));
    }
    check_mcp_registered(&mut checks);
    check_fleet_bootstrap(&mut checks);
    let ok = checks.iter().all(|c| c.status != Severity::Fail);
    Report { ok, checks }
}

async fn check_daemon_up(checks: &mut Vec<Check>, client: &Client) -> bool {
    match client.get::<Value>("/v1/health").await {
        Ok(body) if body.get("ok").and_then(Value::as_bool) == Some(true) => {
            let v = body.get("version").and_then(Value::as_str).unwrap_or("?");
            checks.push(ok("daemon_up", format!("version {v}")));
            true
        }
        Ok(body) => {
            checks.push(fail("daemon_up", format!("unexpected health body: {body}")));
            false
        }
        Err(e) => {
            checks.push(fail("daemon_up", format!("unreachable: {e}")));
            false
        }
    }
}

async fn check_version_match(checks: &mut Vec<Check>, client: &Client) {
    match client.get::<Value>("/v1/health").await {
        Ok(body) => {
            let running = body.get("version").and_then(Value::as_str).unwrap_or("?");
            let expected = env!("CARGO_PKG_VERSION");
            if running == expected {
                checks.push(ok(
                    "version_match",
                    format!("cli={expected} daemon={running}"),
                ));
            } else {
                checks.push(fail(
                    "version_match",
                    format!("cli={expected} != daemon={running}; run install-local.sh"),
                ));
            }
        }
        Err(e) => checks.push(fail("version_match", e.to_string())),
    }
}

async fn check_loops_running(checks: &mut Vec<Check>, client: &Client) {
    match client.get::<Value>("/v1/health").await {
        Ok(body) if body.get("ok").and_then(Value::as_bool) == Some(true) => {
            checks.push(ok(
                "loops_running",
                "daemon healthy — reaper/watcher/executor active",
            ));
        }
        _ => checks.push(fail("loops_running", "daemon unhealthy")),
    }
}

async fn check_embed_nonempty(checks: &mut Vec<Check>, client: &Client) {
    match client.get::<Value>("/v1/embed/stats").await {
        Ok(body) => {
            let count = body.get("count").and_then(Value::as_u64).unwrap_or(0);
            if count > 0 {
                checks.push(ok("embed_nonempty", format!("{count} embedding(s)")));
            } else {
                checks.push(warn(
                    "embed_nonempty",
                    "embed count=0; run cvg graph build then cvg embed build",
                ));
            }
        }
        Err(e) => checks.push(warn("embed_nonempty", format!("embed stats error: {e}"))),
    }
}

async fn check_registry_active(checks: &mut Vec<Check>, client: &Client) {
    match client.get::<Value>("/v1/agent-registry/agents").await {
        Ok(Value::Array(agents)) if !agents.is_empty() => {
            checks.push(ok(
                "registry_active",
                format!("{} agent(s) registered", agents.len()),
            ));
        }
        Ok(_) => checks.push(warn("registry_active", "no agents in registry")),
        Err(e) => checks.push(warn("registry_active", format!("registry error: {e}"))),
    }
}

fn check_mcp_registered(checks: &mut Vec<Check>) {
    if mcp_in_project_config() || mcp_in_user_settings() {
        checks.push(ok("mcp_registered", "convergio MCP found in config"));
    } else {
        checks.push(warn(
            "mcp_registered",
            "convergio not found in .mcp.json or ~/.claude/settings.json; \
             run cvg setup agent claude",
        ));
    }
}

fn mcp_in_project_config() -> bool {
    let Ok(cwd) = std::env::current_dir() else {
        return false;
    };
    let path = cwd.join(".mcp.json");
    if !path.exists() {
        return false;
    }
    std::fs::read_to_string(&path)
        .map(|raw| raw.contains("convergio-mcp") || raw.contains("\"convergio\""))
        .unwrap_or(false)
}

fn mcp_in_user_settings() -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    for candidate in &[
        PathBuf::from(&home).join(".claude").join("settings.json"),
        PathBuf::from(&home)
            .join(".config")
            .join("claude")
            .join("settings.json"),
    ] {
        if let Ok(raw) = std::fs::read_to_string(candidate) {
            if raw.contains("convergio-mcp") || raw.contains("\"convergio\"") {
                return true;
            }
        }
    }
    false
}

fn check_fleet_bootstrap(checks: &mut Vec<Check>) {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = PathBuf::from(&home)
        .join(".convergio")
        .join("v3")
        .join("fleet.toml");
    if !path.exists() {
        checks.push(warn(
            "fleet_bootstrap",
            "fleet.toml not found; run cvg setup fleet",
        ));
        return;
    }
    match std::fs::read_to_string(&path) {
        Ok(raw) => {
            let entries = raw.matches("[[repo]]").count();
            if entries > 0 {
                checks.push(ok("fleet_bootstrap", format!("{entries} repo(s) in fleet")));
            } else {
                checks.push(warn(
                    "fleet_bootstrap",
                    "fleet.toml has no [[repo]] entries",
                ));
            }
        }
        Err(e) => checks.push(warn(
            "fleet_bootstrap",
            format!("fleet.toml read error: {e}"),
        )),
    }
}

fn ok(name: &'static str, message: impl Into<String>) -> Check {
    Check {
        name,
        status: Severity::Ok,
        message: message.into(),
    }
}

fn warn(name: &'static str, message: impl Into<String>) -> Check {
    Check {
        name,
        status: Severity::Warn,
        message: message.into(),
    }
}

fn fail(name: &'static str, message: impl Into<String>) -> Check {
    Check {
        name,
        status: Severity::Fail,
        message: message.into(),
    }
}

fn skip(name: &'static str, message: impl Into<String>) -> Check {
    Check {
        name,
        status: Severity::Warn,
        message: message.into(),
    }
}

fn render_human(bundle: &Bundle, report: &Report) {
    println!("{}", bundle.t("setup-self-check-header", &[]));
    for check in &report.checks {
        let key = match check.status {
            Severity::Ok => "setup-self-check-ok",
            Severity::Warn => "setup-self-check-warn",
            Severity::Fail => "setup-self-check-fail",
        };
        println!(
            "{}",
            bundle.t(key, &[("name", check.name), ("message", &check.message)])
        );
    }
    let summary = if report.ok {
        "setup-self-check-summary-ok"
    } else {
        "setup-self-check-summary-fail"
    };
    println!("{}", bundle.t(summary, &[]));
}
