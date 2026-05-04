//! `cvg validate <plan_id> [--wave N]` — Thor verdict against a
//! plan, plus `--self-test` (P2-7, finding H11) which exercises Thor
//! against a fresh fixture plan + task and reports green/red.
//!
//! Without `--wave`, validation is plan-strict: every task must be
//! `submitted` or `done`. With `--wave N` only that wave is
//! evaluated (T3.06). `--self-test` ignores both arguments, creates
//! a one-shot fixture, runs the verdict, and prints whether the
//! gate pipeline answers — the diagnostic the operator wants at
//! session start instead of having to forge a real task.

use super::Client;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

/// Run the command.
pub async fn run(
    client: &Client,
    plan_id: Option<&str>,
    wave: Option<i64>,
    self_test: bool,
) -> Result<()> {
    if self_test {
        return run_self_test(client).await;
    }
    let plan = plan_id.ok_or_else(|| {
        anyhow!("plan_id is required unless `--self-test` is set; see `cvg validate --help`")
    })?;
    let path = match wave {
        Some(w) => format!("/v1/plans/{plan}/validate?wave={w}"),
        None => format!("/v1/plans/{plan}/validate"),
    };
    let body: Value = client.post(&path, &json!({})).await?;
    println!("{}", serde_json::to_string_pretty(&body)?);
    Ok(())
}

async fn run_self_test(client: &Client) -> Result<()> {
    let plan_title = format!("validate-self-test-{}", short_random());
    println!("cvg validate --self-test");
    println!("  creating fixture plan: {plan_title}");

    let plan: Value = client
        .post("/v1/plans", &json!({ "title": plan_title }))
        .await?;
    let plan_id = plan
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("plan.create response missing id: {plan}"))?
        .to_string();

    let task: Value = client
        .post(
            &format!("/v1/plans/{plan_id}/tasks"),
            &json!({
                "wave": 1,
                "sequence": 1,
                "title": "self-test fixture task",
                "evidence_required": [],
            }),
        )
        .await?;
    let task_id = task
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("task.create response missing id: {task}"))?
        .to_string();
    println!("  fixture task: {}", &task_id[..task_id.len().min(8)]);

    client
        .post::<Value, Value>(
            &format!("/v1/tasks/{task_id}/transition"),
            &json!({ "target": "submitted", "agent_id": "cvg-validate-self-test" }),
        )
        .await?;
    println!("  task → submitted");

    let verdict: Value = client
        .post(&format!("/v1/plans/{plan_id}/validate"), &json!({}))
        .await?;
    println!();
    println!("  verdict:");
    println!("{}", indent(&serde_json::to_string_pretty(&verdict)?, 4));

    let pass = verdict
        .get("verdict")
        .and_then(Value::as_str)
        .map(|v| v.eq_ignore_ascii_case("pass"))
        .unwrap_or(false);

    println!();
    if pass {
        println!("  result: GREEN — Thor responded and approved the fixture.");
        println!("  cleanup: leave the fixture plan in place (status=draft, task=done).");
    } else {
        println!("  result: RED — Thor refused the fixture or did not respond.");
        println!("  inspect the verdict above and `cvg coherence agents` for follow-up.");
        std::process::exit(1);
    }
    Ok(())
}

fn short_random() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:08x}", (nanos as u64) & 0xFFFF_FFFF)
}

fn indent(s: &str, n: usize) -> String {
    let pad = " ".repeat(n);
    s.lines()
        .map(|l| format!("{pad}{l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_random_returns_8_hex_chars() {
        let r = short_random();
        assert_eq!(r.len(), 8);
        assert!(r.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn indent_pads_each_line() {
        let out = indent("a\nb", 2);
        assert_eq!(out, "  a\n  b");
    }
}
