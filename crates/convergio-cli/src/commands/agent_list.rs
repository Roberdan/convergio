//! `cvg agent list` — enriched roll-up of the durable agent registry.
//!
//! Defaults to the `active` view (heartbeat within the threshold,
//! status not in `terminated`/`retired`). `--all` reverts to the
//! historical full dump. `--columns extended` adds capabilities,
//! lease counts, and last audit kind/age.

use super::agent_format::{color_status, maybe_indent_id, relative, relative_ago_opt, truncate};
use super::{Client, OutputMode};
use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use clap::ValueEnum;
use convergio_i18n::Bundle;
use serde::Deserialize;
use serde_json::Value;

/// Column profile chosen via `--columns`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ColumnProfile {
    /// Default: ID, KIND, STATUS, LAST_HB, CLAIMED, LAST_TOPIC, TASK, BRANCH.
    Default,
    /// Default + CAPABILITIES, LEASES, LAST_AUDIT.
    Extended,
}

/// Arguments parsed by clap for `cvg agent list`.
#[derive(Debug, Clone)]
pub struct ListArgs {
    /// Show every agent including terminated/retired.
    pub all: bool,
    /// Threshold (minutes) for the active filter. Default 30.
    pub threshold_min: i64,
    /// Column profile (default vs extended).
    pub columns: ColumnProfile,
}

impl Default for ListArgs {
    fn default() -> Self {
        Self {
            all: false,
            threshold_min: 30,
            columns: ColumnProfile::Default,
        }
    }
}

/// Minimal mirror of the daemon's agent summary payload — keeps
/// the CLI loosely coupled to the Rust struct shape and forgiving
/// of additional fields.
#[derive(Debug, Deserialize)]
struct Summary {
    id: String,
    kind: String,
    status: String,
    #[serde(default)]
    capabilities: Vec<String>,
    #[serde(default)]
    last_heartbeat_at: Option<DateTime<Utc>>,
    #[serde(default)]
    current_task_title: Option<String>,
    #[serde(default)]
    current_task_id: Option<String>,
    #[serde(default)]
    recent_branch: Option<String>,
    #[serde(default)]
    active_leases: i64,
    #[serde(default)]
    last_audit_kind: Option<String>,
    #[serde(default)]
    last_audit_at: Option<DateTime<Utc>>,

    #[serde(default)]
    claimed_tasks: ClaimedTasks,
    #[serde(default)]
    last_topic: Option<LastTopic>,
}

#[derive(Debug, Default, Deserialize)]
struct ClaimedTasks {
    #[serde(default)]
    count: i64,
    #[serde(default)]
    tasks: Vec<ClaimedTask>,
}

#[derive(Debug, Deserialize)]
struct ClaimedTask {
    title: String,
}

#[derive(Debug, Deserialize)]
struct LastTopic {
    topic: String,
    at: DateTime<Utc>,
}

/// Entry point.
pub async fn run(
    client: &Client,
    bundle: &Bundle,
    output: OutputMode,
    args: ListArgs,
) -> Result<()> {
    let raw_arr: Vec<Value> = client.get("/v1/agent-registry/agents/summaries").await?;
    let summaries: Vec<Summary> = raw_arr
        .iter()
        .map(|v| serde_json::from_value::<Summary>(v.clone()))
        .collect::<Result<_, _>>()?;
    let raw: Value = Value::Array(raw_arr);
    let now = Utc::now();
    let (visible, hidden) = filter(summaries, &args, &now);
    match output {
        OutputMode::Human => render_human(bundle, &visible, hidden, &args, &now),
        OutputMode::Json => println!(
            "{}",
            serde_json::to_string_pretty(&select_json(&raw, &visible))?
        ),
        OutputMode::Plain => render_plain(&visible, &now),
    }
    Ok(())
}

fn filter(rows: Vec<Summary>, args: &ListArgs, now: &DateTime<Utc>) -> (Vec<Summary>, usize) {
    if args.all {
        return (sort(rows), 0);
    }
    let cutoff = *now - Duration::minutes(args.threshold_min);
    let total = rows.len();
    let kept: Vec<Summary> = rows
        .into_iter()
        .filter(|s| !is_terminal(&s.status))
        .filter(|s| match s.last_heartbeat_at {
            Some(ts) => ts >= cutoff,
            None => false,
        })
        .collect();
    let hidden = total - kept.len();
    (sort(kept), hidden)
}

fn sort(mut rows: Vec<Summary>) -> Vec<Summary> {
    rows.sort_by_key(|s| std::cmp::Reverse(s.last_heartbeat_at));
    rows
}

fn is_terminal(status: &str) -> bool {
    matches!(status, "terminated" | "retired")
}

fn select_json(raw: &Value, visible: &[Summary]) -> Value {
    let arr = raw.as_array().cloned().unwrap_or_default();
    let ids: std::collections::HashSet<&str> = visible.iter().map(|s| s.id.as_str()).collect();
    Value::Array(
        arr.into_iter()
            .filter(|v| {
                v.get("id")
                    .and_then(Value::as_str)
                    .map(|id| ids.contains(id))
                    .unwrap_or(false)
            })
            .collect(),
    )
}

fn render_plain(rows: &[Summary], now: &DateTime<Utc>) {
    for s in rows {
        let task = s.current_task_title.clone().unwrap_or_else(|| "-".into());
        let branch = s.recent_branch.clone().unwrap_or_else(|| "-".into());
        let hb = match &s.last_heartbeat_at {
            Some(ts) => relative(ts, now),
            None => "-".into(),
        };
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}",
            s.id, s.kind, s.status, hb, task, branch
        );
    }
}

fn render_human(
    bundle: &Bundle,
    rows: &[Summary],
    hidden: usize,
    args: &ListArgs,
    now: &DateTime<Utc>,
) {
    if rows.is_empty() && hidden == 0 {
        println!("{}", bundle.t("agent-list-empty", &[]));
        return;
    }
    let header_key = if args.all {
        "agent-list-header"
    } else {
        "agent-list-header-active"
    };
    println!("{}", bundle.t_n(header_key, rows.len() as i64));
    print_header(bundle, args.columns);
    for s in rows {
        print_row(s, args.columns, now);
    }
    if hidden > 0 && !args.all {
        let hidden_str = hidden.to_string();
        println!(
            "{}",
            bundle.t_n_with(
                "agent-list-stale-hidden",
                hidden as i64,
                &[("count", &hidden_str)]
            )
        );
    }
}

fn print_header(bundle: &Bundle, profile: ColumnProfile) {
    if matches!(profile, ColumnProfile::Default) {
        println!(
            "{:<28} {:<10} {:<11} {:<10} {:<7} {:<20} {:<36} {:<24}",
            bundle.t("agent-list-col-id", &[]),
            bundle.t("agent-list-col-kind", &[]),
            bundle.t("agent-list-col-status", &[]),
            bundle.t("agent-list-col-last-hb", &[]),
            bundle.t("agent-list-col-claimed", &[]),
            bundle.t("agent-list-col-last-topic", &[]),
            bundle.t("agent-list-col-task", &[]),
            bundle.t("agent-list-col-branch", &[]),
        );
    } else {
        println!(
            "{:<28} {:<10} {:<11} {:<10} {:<7} {:<18} {:<30} {:<20} {:<24} {:<8} {:<22}",
            bundle.t("agent-list-col-id", &[]),
            bundle.t("agent-list-col-kind", &[]),
            bundle.t("agent-list-col-status", &[]),
            bundle.t("agent-list-col-last-hb", &[]),
            bundle.t("agent-list-col-claimed", &[]),
            bundle.t("agent-list-col-last-topic", &[]),
            bundle.t("agent-list-col-task", &[]),
            bundle.t("agent-list-col-branch", &[]),
            bundle.t("agent-list-col-capabilities", &[]),
            bundle.t("agent-list-col-leases", &[]),
            bundle.t("agent-list-col-last-audit", &[]),
        );
    }
}

fn print_row(s: &Summary, profile: ColumnProfile, now: &DateTime<Utc>) {
    let id = maybe_indent_id(&s.id, &s.kind);
    let kind = s.kind.clone();
    let status = color_status(&s.status);
    let hb = relative_ago_opt(s.last_heartbeat_at.as_ref(), now);
    let mut task = s
        .current_task_title
        .clone()
        .or_else(|| s.claimed_tasks.tasks.first().map(|t| t.title.clone()))
        .or_else(|| s.current_task_id.clone())
        .map(|t| truncate(&t, 38))
        .unwrap_or_else(|| "-".into());
    if s.claimed_tasks.count > 1 {
        task = truncate(&format!("{task} (+{})", s.claimed_tasks.count - 1), 38);
    }
    let branch = s
        .recent_branch
        .clone()
        .map(|b| truncate(&b, 22))
        .unwrap_or_else(|| "-".into());
    let claimed = format!("{}", s.claimed_tasks.count);
    let topic = match &s.last_topic {
        Some(t) => {
            let age = relative(&t.at, now);
            truncate(&format!("{} {age}", t.topic), 18)
        }
        None => "-".into(),
    };
    if matches!(profile, ColumnProfile::Default) {
        println!(
            "{id:<36} {kind:<10} {status:<20} {hb:<10} {claimed:<7} {topic:<20} {task:<36} {branch:<24}"
        );
    } else {
        let caps = truncate(&s.capabilities.join(","), 18);
        let leases = format!("{}", s.active_leases);
        let audit = match &s.last_audit_kind {
            Some(k) => format!(
                "{} ({})",
                truncate(k, 14),
                relative_ago_opt(s.last_audit_at.as_ref(), now)
            ),
            None => "-".into(),
        };
        println!(
            "{id:<36} {kind:<10} {status:<20} {hb:<10} {claimed:<7} {topic:<18} {:<30} {branch:<20} {caps:<24} {leases:<8} {audit:<22}",
            truncate(&task, 28)
        );
    }
}
