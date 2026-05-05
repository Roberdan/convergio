//! Output rendering for `cvg session register-and-poll`.
//!
//! Split out of the parent module to keep both files under the
//! 300-line cap (CONSTITUTION § 13). Three views: human (Fluent
//! bundle), JSON (pretty), plain (TSV-ish for shell pipelines).

use crate::register_and_poll::{PlanRef, SessionReport};
use crate::OutputMode;
use anyhow::Result;
use convergio_i18n::Bundle;
use serde_json::{json, Value};

/// Render a session report to stdout in the requested format.
pub fn render(output: OutputMode, bundle: &Bundle, report: &SessionReport<'_>) -> Result<()> {
    match output {
        OutputMode::Json => render_json(report),
        OutputMode::Plain => {
            render_plain(report);
            Ok(())
        }
        OutputMode::Human => {
            render_human(bundle, report);
            Ok(())
        }
    }
}

fn render_json(report: &SessionReport<'_>) -> Result<()> {
    let v = json!({
        "agent": report.agent,
        "heartbeat": report.heartbeat,
        "active_plans": report.plans.iter().map(plan_json).collect::<Vec<_>>(),
        "direct_messages": report.direct.iter().map(envelope_json).collect::<Vec<_>>(),
        "plan_announcements": report
            .announcements
            .iter()
            .map(envelope_json)
            .collect::<Vec<_>>(),
        "acked_direct": report.acked_direct,
    });
    println!("{}", serde_json::to_string_pretty(&v)?);
    Ok(())
}

fn plan_json(p: &PlanRef) -> Value {
    json!({"id": p.id, "title": p.title, "status": p.status})
}

fn envelope_json(env: &(String, Vec<Value>)) -> Value {
    json!({"plan_id": env.0, "messages": env.1})
}

fn render_plain(report: &SessionReport<'_>) {
    let id = field(&report.agent, "id");
    let status = field(&report.heartbeat, "status");
    println!("registered\t{id}");
    println!("heartbeat\t{status}");
    for p in report.plans {
        println!("plan\t{}\t{}", p.id, p.status);
    }
    for (p, m) in report.direct {
        println!("direct\t{p}\t{}", m.len());
    }
    for (p, m) in report.announcements {
        println!("announcement\t{p}\t{}", m.len());
    }
    println!("acked_direct\t{}", report.acked_direct);
}

fn render_human(bundle: &Bundle, report: &SessionReport<'_>) {
    let id = field(&report.agent, "id");
    let kind = field(&report.agent, "kind");
    let host = field(&report.agent, "host");
    let status = field(&report.heartbeat, "status");
    println!("{}", bundle.t("session-register-poll-header", &[]));
    println!(
        "{}",
        bundle.t(
            "session-register-poll-registered",
            &[("id", id), ("kind", kind), ("host", host)],
        )
    );
    println!(
        "{}",
        bundle.t("session-register-poll-heartbeat", &[("status", status)])
    );
    println!(
        "{}",
        bundle.t_n(
            "session-register-poll-plans-header",
            report.plans.len() as i64
        )
    );
    for p in report.plans {
        println!(
            "{}",
            bundle.t(
                "session-register-poll-plan-line",
                &[("id", &p.id), ("title", &p.title)],
            )
        );
    }
    print_envelopes(bundle, "session-register-poll-direct-header", report.direct);
    if report.acked_direct > 0 {
        println!(
            "{}",
            bundle.t_n("session-register-poll-acked", report.acked_direct as i64,)
        );
    }
    print_envelopes(
        bundle,
        "session-register-poll-announcements-header",
        report.announcements,
    );
}

fn print_envelopes(bundle: &Bundle, header_key: &str, envelopes: &[(String, Vec<Value>)]) {
    let total: usize = envelopes.iter().map(|(_, m)| m.len()).sum();
    println!("{}", bundle.t_n(header_key, total as i64));
    for (plan_id, msgs) in envelopes {
        for m in msgs {
            print_message_line(bundle, plan_id, m);
        }
    }
}

fn print_message_line(bundle: &Bundle, plan_id: &str, m: &Value) {
    let seq = m
        .get("seq")
        .and_then(Value::as_i64)
        .map(|n| n.to_string())
        .unwrap_or_else(|| "?".to_string());
    let topic = m.get("topic").and_then(Value::as_str).unwrap_or("?");
    let sender = m.get("sender").and_then(Value::as_str).unwrap_or("-");
    println!(
        "{}",
        bundle.t(
            "session-register-poll-message-line",
            &[
                ("plan", plan_id),
                ("seq", &seq),
                ("topic", topic),
                ("sender", sender),
            ],
        )
    );
}

fn field<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("?")
}
