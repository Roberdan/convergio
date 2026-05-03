//! `cvg agent spawn` — start a vendor-CLI agent against a task.
//!
//! Pulls the task + plan + (optionally) the graph context-pack from
//! the daemon, hands them to the right [`convergio_runner::Runner`],
//! then either prints the prepared command (`--dry-run`) or executes
//! it inline. The vendor CLI inherits the operator's existing auth;
//! Convergio never sees the API key (ADR-0032).

use super::agent_spawn_wire::TaskWire;
use super::{Client, OutputMode};
use anyhow::{anyhow, Context as _, Result};
use convergio_durability::Task;
use convergio_runner::{
    for_kind_with_registry, PermissionProfile, RunnerKind, RunnerRegistry, SpawnContext,
};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::str::FromStr;
use std::thread;

/// All operator-controlled flags from `cvg agent spawn`. Bundling
/// them in one struct keeps the dispatcher under clippy's
/// `too_many_arguments` lint and makes the call-site readable.
pub struct SpawnArgs {
    /// `--task` value.
    pub task_id: String,
    /// `--runner` wire string.
    pub runner: String,
    /// Override stable agent id (default derived from kind+task).
    pub agent_id: Option<String>,
    /// Override working directory (default cwd).
    pub cwd: Option<PathBuf>,
    /// `--max-budget-usd` (Claude only).
    pub max_budget_usd: Option<f32>,
    /// `--profile` value (`standard` / `read_only` / `sandbox`).
    pub profile: String,
    /// `--dry-run` toggle.
    pub dry_run: bool,
}

/// Run `cvg agent spawn`.
pub async fn run(client: &Client, output: OutputMode, args: SpawnArgs) -> Result<()> {
    let SpawnArgs {
        task_id,
        runner,
        agent_id,
        cwd,
        max_budget_usd,
        profile,
        dry_run,
    } = args;
    let kind = RunnerKind::from_str(&runner).context("parse --runner")?;
    let profile = PermissionProfile::from_str(&profile).map_err(anyhow::Error::msg)?;
    let task = fetch_task(client, &task_id).await?;
    let plan = fetch_plan(client, &task.plan_id).await?;
    let plan_title = plan
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("(untitled)");

    let graph_context = fetch_graph_context(client, &task_id).await;
    let graph_context_ref = graph_context.as_deref();

    let agent = agent_id
        .clone()
        .unwrap_or_else(|| default_agent_id(&kind, &task.id));
    let cwd_ref = cwd
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let ctx = SpawnContext {
        task: &task,
        plan_id: &task.plan_id,
        plan_title,
        daemon_url: client.base(),
        agent_id: &agent,
        graph_context: graph_context_ref,
        cwd: &cwd_ref,
        max_budget_usd,
        profile,
    };
    let registry = RunnerRegistry::load_default().context("load runner registry")?;
    let prepared = for_kind_with_registry(&kind, &registry)
        .and_then(|r| r.prepare(&ctx))
        .map_err(anyhow::Error::msg)?;

    if dry_run {
        emit_dry_run(output, &prepared, &kind, &agent)?;
        return Ok(());
    }

    let (mut cmd, prompt) = prepared.into_std_command();
    let summary = format!(
        "spawning {kind} for task {} (agent={agent}, cwd={}) ...",
        task.id,
        cwd_ref.display(),
    );
    if matches!(output, OutputMode::Human | OutputMode::Plain) {
        eprintln!("{summary}");
    }

    let mut child = cmd.spawn().with_context(|| {
        format!(
            "failed to spawn vendor CLI `{}` — is it installed and on PATH?",
            kind.vendor
        )
    })?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .context("write prompt to vendor CLI stdin")?;
        // Closing stdin signals end-of-input to the vendor CLI; both
        // `claude -p --input-format text` and `copilot -p` need this
        // to start producing output.
        drop(stdin);
    }
    // Pipe stdout/stderr to the operator's terminal in real time so a
    // long-running session is observable. Each line is forwarded as
    // it arrives — for `claude --output-format stream-json` that means
    // one JSON event per turn, which the operator can `jq` on.
    let stdout_handle = child
        .stdout
        .take()
        .map(|s| thread::spawn(move || forward_lines(s, "stdout")));
    let stderr_handle = child
        .stderr
        .take()
        .map(|s| thread::spawn(move || forward_lines(s, "stderr")));
    let status = child.wait().context("wait for vendor CLI to exit")?;
    if let Some(h) = stdout_handle {
        let _ = h.join();
    }
    if let Some(h) = stderr_handle {
        let _ = h.join();
    }
    if !status.success() {
        return Err(anyhow!(
            "vendor CLI exited with non-zero status: {:?}",
            status.code()
        ));
    }
    Ok(())
}

fn forward_lines<R: std::io::Read>(reader: R, channel: &'static str) {
    let buf = BufReader::new(reader);
    for line in buf.lines().map_while(std::result::Result::ok) {
        match channel {
            "stderr" => eprintln!("{line}"),
            _ => println!("{line}"),
        }
    }
}

fn emit_dry_run(
    output: OutputMode,
    prepared: &convergio_runner::PreparedCommand,
    kind: &RunnerKind,
    agent: &str,
) -> Result<()> {
    match output {
        OutputMode::Json => {
            let argv: Vec<String> = prepared
                .args
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect();
            let v = serde_json::json!({
                "kind": kind.to_string(),
                "agent_id": agent,
                "program": prepared.program.to_string_lossy(),
                "args": argv,
                "cwd": prepared.cwd.display().to_string(),
                "stdin_prompt_bytes": prepared.stdin_prompt.len(),
                "stdin_prompt": prepared.stdin_prompt,
            });
            println!("{}", serde_json::to_string_pretty(&v)?);
        }
        _ => {
            println!("# cvg agent spawn — dry run");
            println!("kind:  {kind}");
            println!("agent: {agent}");
            println!("cwd:   {}", prepared.cwd.display());
            print!("argv:  {}", prepared.program.to_string_lossy());
            for a in &prepared.args {
                print!(" {}", a.to_string_lossy());
            }
            println!();
            println!("\n--- prompt ({} bytes) ---", prepared.stdin_prompt.len());
            println!("{}", prepared.stdin_prompt);
        }
    }
    Ok(())
}

/// Build a default agent id like `claude-sonnet-t1234abc`.
fn default_agent_id(kind: &RunnerKind, task_id: &str) -> String {
    let slug = task_id.get(..7).unwrap_or(task_id);
    format!("{}-{}-{}", kind.vendor, kind.model, slug)
}

async fn fetch_task(client: &Client, id: &str) -> Result<Task> {
    let raw: Value = client.get(&format!("/v1/tasks/{id}")).await?;
    let row: TaskWire =
        serde_json::from_value(raw.clone()).context("decode task from /v1/tasks")?;
    Ok(row.into_task())
}

async fn fetch_plan(client: &Client, id: &str) -> Result<Value> {
    client.get(&format!("/v1/plans/{id}")).await
}

/// Best-effort: returns `None` when the graph is not built or the
/// route fails. The runner copes with absent context.
async fn fetch_graph_context(client: &Client, task_id: &str) -> Option<String> {
    let raw: Value = client
        .get(&format!("/v1/graph/for-task/{task_id}"))
        .await
        .ok()?;
    raw.get("text")
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .or_else(|| serde_json::to_string_pretty(&raw).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_agent_id_uses_kind_and_slug() {
        let kind = RunnerKind::claude_sonnet();
        let id = default_agent_id(&kind, "abcdef1234567890");
        assert_eq!(id, "claude-sonnet-abcdef1");
    }

    #[test]
    fn default_agent_id_handles_short_task_id() {
        let kind = RunnerKind::copilot_gpt();
        let id = default_agent_id(&kind, "abc");
        assert_eq!(id, "copilot-gpt-5.2-abc");
    }
}
