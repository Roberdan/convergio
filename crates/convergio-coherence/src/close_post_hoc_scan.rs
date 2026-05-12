//! Scanner half of `cvg coherence close-post-hoc`. Walks the daemon
//! audit chain via paginated `GET /v1/audit/events`, filters
//! `transition = task.closed_post_hoc` rows in the requested window,
//! enriches each row with the task title.

use super::close_post_hoc::Row;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub(super) struct AuditEvent {
    pub seq: i64,
    pub entity_id: String,
    pub transition: String,
    #[serde(default)]
    pub payload: Option<String>,
    #[serde(default)]
    pub agent_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

pub(super) async fn scan_audit(
    client: &reqwest::Client,
    daemon: &str,
    cutoff: DateTime<Utc>,
) -> Result<Vec<Row>> {
    const PAGE: usize = 500;
    let mut hits: Vec<Row> = Vec::new();
    let mut since_seq: i64 = 0;
    loop {
        let url = format!("{daemon}/v1/audit/events?since={since_seq}&limit={PAGE}");
        let page: Vec<AuditEvent> = match client.get(&url).send().await {
            Ok(r) if r.status().is_success() => r.json().await.unwrap_or_default(),
            _ => return Ok(hits),
        };
        if page.is_empty() {
            break;
        }
        let max_seq = page.iter().map(|e| e.seq).max().unwrap_or(since_seq);
        for ev in &page {
            if ev.transition != "task.closed_post_hoc" {
                continue;
            }
            if ev.created_at < cutoff {
                continue;
            }
            hits.push(row_from(ev));
        }
        if max_seq <= since_seq || page.len() < PAGE {
            break;
        }
        since_seq = max_seq;
    }
    hits.sort_by_key(|r| r.created_at);
    Ok(hits)
}

pub(super) fn row_from(ev: &AuditEvent) -> Row {
    let payload: Value = ev
        .payload
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null);
    Row {
        task_id: ev.entity_id.clone(),
        task_title: String::new(),
        plan_id: payload
            .get("plan_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        agent_id: ev.agent_id.clone().unwrap_or_default(),
        reason: payload
            .get("reason")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        created_at: ev.created_at,
    }
}

pub(super) async fn enrich_titles(
    client: &reqwest::Client,
    daemon: &str,
    rows: Vec<Row>,
) -> Vec<Row> {
    let mut enriched = Vec::with_capacity(rows.len());
    for mut r in rows {
        let url = format!("{daemon}/v1/tasks/{}", r.task_id);
        if let Ok(resp) = client.get(&url).send().await {
            if resp.status().is_success() {
                if let Ok(t) = resp.json::<Value>().await {
                    r.task_title = t
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    if r.plan_id.is_empty() {
                        r.plan_id = t
                            .get("plan_id")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string();
                    }
                }
            }
        }
        enriched.push(r);
    }
    enriched
}

pub(super) fn aggregate<F: Fn(&Row) -> String>(rows: &[Row], key: F) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for r in rows {
        let k = key(r);
        if k.is_empty() {
            continue;
        }
        *counts.entry(k).or_insert(0) += 1;
    }
    let mut v: Vec<_> = counts.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

/// Parse `--since`. Accepts `Nd` (days) or `Nh` (hours). Anything
/// else collapses to "since 7 days ago"; the raw string is kept in
/// the report so the operator sees what was used.
pub(super) fn parse_since(s: &str) -> Result<DateTime<Utc>> {
    let now = Utc::now();
    if let Some(num) = s.strip_suffix('d') {
        let n: i64 = num
            .parse()
            .context("--since: expected integer before 'd'")?;
        return Ok(now - Duration::days(n));
    }
    if let Some(num) = s.strip_suffix('h') {
        let n: i64 = num
            .parse()
            .context("--since: expected integer before 'h'")?;
        return Ok(now - Duration::hours(n));
    }
    Ok(now - Duration::days(7))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ev(transition: &str, days_ago: i64, payload: Value) -> AuditEvent {
        AuditEvent {
            seq: 1,
            entity_id: "task-1".into(),
            transition: transition.into(),
            payload: Some(payload.to_string()),
            agent_id: Some("agent-x".into()),
            created_at: Utc::now() - Duration::days(days_ago),
        }
    }

    #[test]
    fn row_extracts_reason_and_plan_from_payload() {
        let e = ev(
            "task.closed_post_hoc",
            1,
            json!({"reason": "shipped pre-Thor", "plan_id": "plan-1"}),
        );
        let r = row_from(&e);
        assert_eq!(r.reason, "shipped pre-Thor");
        assert_eq!(r.plan_id, "plan-1");
        assert_eq!(r.agent_id, "agent-x");
    }

    #[test]
    fn aggregate_orders_descending_by_count_then_id() {
        let rows = vec![
            row_from(&ev("task.closed_post_hoc", 0, json!({"plan_id": "a"}))),
            row_from(&ev("task.closed_post_hoc", 0, json!({"plan_id": "b"}))),
            row_from(&ev("task.closed_post_hoc", 0, json!({"plan_id": "a"}))),
        ];
        let by_plan = aggregate(&rows, |r| r.plan_id.clone());
        assert_eq!(by_plan, vec![("a".into(), 2), ("b".into(), 1)]);
    }

    #[test]
    fn parse_since_accepts_days_and_hours() {
        let now = Utc::now();
        let one_day = parse_since("1d").unwrap();
        assert!((now - one_day).num_hours() >= 23 && (now - one_day).num_hours() <= 25);
        let three_h = parse_since("3h").unwrap();
        assert!((now - three_h).num_minutes() >= 175 && (now - three_h).num_minutes() <= 185);
    }

    #[test]
    fn parse_since_falls_back_to_seven_days_on_garbage() {
        let now = Utc::now();
        let fallback = parse_since("garbage").unwrap();
        assert!((now - fallback).num_days() >= 6 && (now - fallback).num_days() <= 8);
    }

    // Regression test for audit finding `close_post_hoc_scan.rs:36`:
    // when the audit page fetch failed, scan_audit returned a partial
    // clean Ok(hits) instead of surfacing the failure, so the verifier
    // could report "no close-post-hoc rows" even when it never reached
    // the daemon. With the fix the HTTP failure propagates as Err.
    #[tokio::test]
    async fn scan_audit_propagates_page_fetch_failure() {
        use axum::{routing::get, Router};
        use tokio::net::TcpListener;

        let router = Router::new().route(
            "/v1/audit/events",
            get(|| async { (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "boom") }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let base = format!("http://{addr}");
        let client = reqwest::Client::new();
        let res = scan_audit(&client, &base, Utc::now() - Duration::days(7)).await;
        assert!(
            res.is_err(),
            "expected scan_audit to surface HTTP 500 as Err, got {res:?}"
        );
    }
}
