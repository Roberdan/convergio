//! Rendering for `cvg discover`. Split out of `discover.rs` so the
//! command file keeps headroom under the 300-line cap (T828d03c
//! audit follow-up).

use super::agent_format::{relative_ago_opt, truncate};
use chrono::{DateTime, Utc};
use convergio_i18n::Bundle;
use serde_json::Value;

/// Render the discover snapshot in localized human form.
pub(super) fn render_human(
    bundle: &Bundle,
    now: &DateTime<Utc>,
    since: &str,
    me: &str,
    peers: &[Value],
    topics: &[Value],
    plans: &[Value],
) {
    println!(
        "{} ({})",
        bundle.t("discover-header", &[("at", &now.to_rfc3339())]),
        me
    );
    println!();
    println!("{}", bundle.t("discover-active-peers", &[("since", since)]));
    if peers.is_empty() {
        println!("  {}", bundle.t("discover-empty-peers", &[]));
    }
    for p in peers {
        render_peer_human(p, now);
    }
    println!();
    println!("{}", bundle.t("discover-recent-bus", &[]));
    if topics.is_empty() {
        println!("  {}", bundle.t("discover-empty-bus", &[]));
    }
    for t in topics {
        render_topic_human(t, now);
    }
    println!();
    println!("{}", bundle.t("discover-your-plans", &[]));
    if plans.is_empty() {
        println!("  {}", bundle.t("discover-empty-plans", &[]));
    }
    for p in plans {
        render_plan_human(p);
    }
}

fn render_peer_human(p: &Value, now: &DateTime<Utc>) {
    let id = p["id"].as_str().unwrap_or("-");
    let kind = p["kind"].as_str().unwrap_or("-");
    let status = p["status"].as_str().unwrap_or("-");
    let caps = caps_string(p);
    let task = p["current_task_id"].as_str().unwrap_or("-");
    let hb = parse_dt(&p["last_heartbeat_at"]);
    println!(
        "  {:<28} {:<10} {:<11} {:<10} {:<32} {}",
        truncate(id, 28),
        kind,
        status,
        relative_ago_opt(hb.as_ref(), now),
        truncate(&caps, 32),
        truncate(task, 32)
    );
}

fn render_topic_human(t: &Value, now: &DateTime<Utc>) {
    let pid = t["plan_id"].as_str().unwrap_or("");
    println!(
        "  {:<40} {:<8} plan {} {}",
        truncate(t["topic"].as_str().unwrap_or(""), 40),
        t["count"].as_i64().unwrap_or(0),
        &pid[..8.min(pid.len())],
        relative_ago_opt(parse_dt(&t["last_at"]).as_ref(), now)
    );
}

fn render_plan_human(p: &Value) {
    let pid = p["plan_id"].as_str().unwrap_or("");
    println!(
        "  {} {:<8} {:<48} open={}",
        &pid[..8.min(pid.len())],
        p["status"].as_str().unwrap_or(""),
        truncate(p["title"].as_str().unwrap_or(""), 48),
        p["your_tasks_open"].as_i64().unwrap_or(0)
    );
}

/// Tab-separated rendering for shell pipelines.
pub(super) fn render_plain(
    peers: &[Value],
    topics: &[Value],
    plans: &[Value],
    now: &DateTime<Utc>,
) {
    for p in peers {
        let caps = caps_string(p);
        println!(
            "peer\t{}\t{}\t{}\t{}\t{}",
            p["id"].as_str().unwrap_or(""),
            p["kind"].as_str().unwrap_or(""),
            p["status"].as_str().unwrap_or(""),
            relative_ago_opt(parse_dt(&p["last_heartbeat_at"]).as_ref(), now),
            caps
        );
    }
    for t in topics {
        println!(
            "topic\t{}\t{}\t{}\t{}",
            t["topic"].as_str().unwrap_or(""),
            t["plan_id"].as_str().unwrap_or(""),
            t["count"].as_i64().unwrap_or(0),
            relative_ago_opt(parse_dt(&t["last_at"]).as_ref(), now)
        );
    }
    for p in plans {
        println!(
            "plan\t{}\t{}\t{}\t{}",
            p["plan_id"].as_str().unwrap_or(""),
            p["status"].as_str().unwrap_or(""),
            p["your_tasks_open"].as_i64().unwrap_or(0),
            p["title"].as_str().unwrap_or("")
        );
    }
}

fn caps_string(p: &Value) -> String {
    p["capabilities"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

fn parse_dt(v: &Value) -> Option<DateTime<Utc>> {
    v.as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc))
}
