//! Opus-backed planner.
//!
//! Spawns `claude -p --model opus --output-format json` (vendor CLI
//! only — ADR-0032), pipes a structured prompt on stdin, parses the
//! JSON response, and persists plan + tasks. ADR-0036.
//!
//! Pure functions ([`build_prompt`], [`parse_response`],
//! [`extract_json_object`]) carry the testable logic; the spawn is
//! kept thin so it can be skipped in CI (claude is not on PATH).

use crate::error::{PlannerError, Result};
use crate::schema::PlanShape;
use convergio_durability::{Durability, NewPlan, NewTask};
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// Run the Opus planner end-to-end: spawn, parse, persist.
pub async fn solve(durability: &Durability, mission: &str) -> Result<String> {
    let trimmed = mission.trim();
    if trimmed.is_empty() {
        return Err(PlannerError::EmptyMission);
    }
    let prompt = build_prompt(trimmed);
    let raw = spawn_claude_opus(&prompt).await?;
    let shape = parse_response(&raw)?;
    persist(durability, shape).await
}

/// Build the planner prompt. Pure — used by tests to assert content.
pub fn build_prompt(mission: &str) -> String {
    let schema = PlanShape::JSON_SCHEMA_HINT;
    format!(
        "You are the Convergio planner. Convergio executes complex software work by \
         dispatching tasks to vendor CLIs (claude, copilot, qwen, codex, gemini). \
         Your job: produce ONE JSON object describing a plan for the mission below.\n\n\
         Optimize for, in order:\n\
           1. PR cardinality — small reviewable PRs, ideally one task per PR.\n\
           2. Cost vs quality — cheap tasks → claude:sonnet or copilot:gpt-5.2-mini; \
              reasoning-heavy tasks → claude:opus.\n\
           3. Safety — read-only tasks → profile=read_only; mutating tasks → \
              profile=standard. Never use sandbox in production.\n\
           4. Wave parallelism — independent tasks share a wave; dependent tasks \
              go in subsequent waves.\n\
           5. Crisp evidence — list file paths, command outputs, or test names \
              that prove the task is done.\n\n\
         Output schema (JSON object, NOTHING ELSE — no prose, no markdown fence):\n{schema}\n\n\
         Mission:\n{mission}"
    )
}

/// Parse the assistant response. Accepts either a bare plan JSON
/// or the `claude -p --output-format json` envelope (extracts
/// `.result` then parses). Returns `OpusOutputInvalid` on drift.
pub fn parse_response(raw: &str) -> Result<PlanShape> {
    let inner = extract_inner_payload(raw)?;
    let cleaned = extract_json_object(&inner)
        .ok_or_else(|| PlannerError::OpusOutputInvalid("no JSON object found".into()))?;
    let shape: PlanShape = serde_json::from_str(cleaned)
        .map_err(|e| PlannerError::OpusOutputInvalid(format!("parse PlanShape: {e}")))?;
    shape.validate()?;
    Ok(shape)
}

/// Extract `.result` from the claude `--output-format json`
/// envelope, falling back to the raw input when the envelope is
/// not present (e.g. the model already returned bare JSON).
fn extract_inner_payload(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if let Ok(envelope) = serde_json::from_str::<serde_json::Value>(trimmed) {
        if let Some(result) = envelope.get("result").and_then(|v| v.as_str()) {
            return Ok(result.to_string());
        }
    }
    Ok(trimmed.to_string())
}

/// Find the first `{ ... }` JSON object in `s` (greedy by depth
/// counting). Tolerates leading prose / markdown fences from the
/// model.
fn extract_json_object(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let start = bytes.iter().position(|&b| b == b'{')?;
    let mut depth: i32 = 0;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

async fn spawn_claude_opus(prompt: &str) -> Result<String> {
    let mut child = Command::new("claude")
        .args([
            "-p",
            "--model",
            "opus",
            "--output-format",
            "json",
            "--input-format",
            "text",
            "--permission-mode",
            "plan",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| PlannerError::OpusSpawn(e.to_string()))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(prompt.as_bytes())
            .await
            .map_err(|e| PlannerError::OpusSpawn(format!("write stdin: {e}")))?;
        drop(stdin);
    }

    let out = child
        .wait_with_output()
        .await
        .map_err(|e| PlannerError::OpusSpawn(format!("wait: {e}")))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        let truncated: String = stderr.chars().take(2048).collect();
        return Err(PlannerError::OpusExited {
            status: out.status.code().unwrap_or(-1),
            stderr: truncated,
        });
    }
    String::from_utf8(out.stdout)
        .map_err(|e| PlannerError::OpusOutputInvalid(format!("stdout not utf-8: {e}")))
}

async fn persist(durability: &Durability, shape: PlanShape) -> Result<String> {
    let plan = durability
        .create_plan(NewPlan {
            title: shape.title,
            description: shape.description,
            project: None,
        })
        .await?;
    for t in shape.tasks {
        durability
            .create_task(
                &plan.id,
                NewTask {
                    wave: t.wave,
                    sequence: t.sequence,
                    title: t.title,
                    description: t.description,
                    evidence_required: t.evidence_required,
                    runner_kind: t.runner_kind,
                    profile: t.profile,
                    max_budget_usd: t.max_budget_usd,
                },
            )
            .await?;
    }
    Ok(plan.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt_includes_mission_and_schema() {
        let p = build_prompt("Add a /v1/health endpoint with timing data");
        assert!(p.contains("Add a /v1/health endpoint"));
        assert!(p.contains("PR cardinality"));
        assert!(p.contains("runner_kind"));
        assert!(p.contains("evidence_required"));
    }

    #[test]
    fn extract_json_object_handles_prose_prefix() {
        let s = "Here is your plan:\n\n{\"title\":\"x\",\"tasks\":[]}\n\nLet me know.";
        let obj = extract_json_object(s).unwrap();
        assert_eq!(obj, "{\"title\":\"x\",\"tasks\":[]}");
    }

    #[test]
    fn extract_json_object_handles_nested_braces() {
        let s = r#"{"a":{"b":{"c":1}}, "d":2}"#;
        let obj = extract_json_object(s).unwrap();
        assert_eq!(obj, s);
    }

    #[test]
    fn parse_response_accepts_bare_plan_json() {
        let raw = r#"
        {
          "title": "Add health endpoint",
          "description": null,
          "tasks": [
            {
              "wave": 1,
              "sequence": 1,
              "title": "Wire route",
              "description": null,
              "evidence_required": ["tests pass"],
              "runner_kind": "claude:sonnet",
              "profile": "standard",
              "max_budget_usd": 0.25
            }
          ]
        }"#;
        let shape = parse_response(raw).unwrap();
        assert_eq!(shape.title, "Add health endpoint");
        assert_eq!(shape.tasks.len(), 1);
        assert_eq!(shape.tasks[0].runner_kind.as_deref(), Some("claude:sonnet"));
    }

    #[test]
    fn parse_response_unwraps_claude_envelope() {
        let envelope = serde_json::json!({
            "type": "result",
            "result": "{\"title\":\"x\",\"tasks\":[{\"wave\":1,\"sequence\":1,\"title\":\"t\",\"evidence_required\":[]}]}"
        });
        let raw = envelope.to_string();
        let shape = parse_response(&raw).unwrap();
        assert_eq!(shape.title, "x");
    }

    #[test]
    fn parse_response_rejects_garbage() {
        let err = parse_response("nothing useful here").unwrap_err();
        assert!(matches!(err, PlannerError::OpusOutputInvalid(_)));
    }

    #[test]
    fn parse_response_rejects_empty_tasks() {
        let raw = r#"{"title":"x","tasks":[]}"#;
        let err = parse_response(raw).unwrap_err();
        assert!(matches!(err, PlannerError::OpusOutputInvalid(_)));
    }
}
