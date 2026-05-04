//! Rendering helpers for `cvg bus tail` / `cvg bus list`.
//!
//! Holds the envelope shape, output enum, and the SSE chunk parser
//! used by [`super::bus_tail`]. Kept separate so the streaming
//! consumer file stays under the 300-line cap.

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, Write};

use super::OutputMode;

/// Output flavour for `bus tail` and `bus list`.
///
/// Mirrors the global [`OutputMode`] but stays a separate type so the
/// future addition of bus-only flavours (e.g. `--output csv`) does not
/// leak into every other command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum BusOutput {
    /// Localized human output, e.g. `[12:01:02] @sender → topic: kind (seq 7)`.
    Human,
    /// One JSON object per line — the raw envelope.
    Json,
    /// Tab-separated `seq\tsender\ttopic\tkind` for shell pipelines.
    Plain,
}

impl BusOutput {
    /// Promote the global mode to a bus-specific one.
    pub fn from_global(m: OutputMode) -> Self {
        match m {
            OutputMode::Human => Self::Human,
            OutputMode::Json => Self::Json,
            OutputMode::Plain => Self::Plain,
        }
    }
}

/// Minimal envelope shape we render — extra fields on the wire are
/// preserved by `Value` for JSON output.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Envelope {
    /// Monotonic per-plan cursor.
    pub seq: i64,
    /// Topic name, e.g. `coordination/agents`.
    pub topic: String,
    /// Sender id; system-emitted messages have no sender.
    #[serde(default)]
    pub sender: Option<String>,
    /// Raw payload — bus messages are free-form JSON.
    pub payload: Value,
    /// Server-set timestamp (RFC3339).
    pub created_at: String,
}

/// Render one envelope according to `output`. Writes to stdout and
/// flushes so streaming output appears live.
pub fn render(env: &Envelope, output: BusOutput) -> Result<()> {
    let stdout = io::stdout();
    let mut h = stdout.lock();
    match output {
        BusOutput::Json => {
            writeln!(h, "{}", serde_json::to_string(env)?)?;
        }
        BusOutput::Plain => {
            let kind = payload_kind(&env.payload).unwrap_or("-");
            writeln!(
                h,
                "{}\t{}\t{}\t{}",
                env.seq,
                env.sender.as_deref().unwrap_or("-"),
                env.topic,
                kind
            )?;
        }
        BusOutput::Human => {
            let ts = short_time(&env.created_at);
            let kind = payload_kind(&env.payload)
                .map(str::to_string)
                .unwrap_or_else(|| short_payload(&env.payload));
            writeln!(
                h,
                "[{ts}] @{sender} \u{2192} {topic}: {kind} (seq {seq})",
                sender = env.sender.as_deref().unwrap_or("system"),
                topic = env.topic,
                seq = env.seq,
            )?;
        }
    }
    h.flush()?;
    Ok(())
}

/// Parse zero or more complete `event: ...\ndata: ...\n\n` blocks
/// out of `buf`. Consumed bytes are removed; partial trailing blocks
/// stay in the buffer for the next chunk.
pub fn drain_events(buf: &mut String) -> Vec<Envelope> {
    let mut out = Vec::new();
    while let Some(idx) = buf.find("\n\n") {
        let block: String = buf.drain(..idx + 2).collect();
        let mut data: Option<String> = None;
        for line in block.lines() {
            // SSE keepalive comments start with ':' — skip them.
            if line.starts_with(':') {
                continue;
            }
            if let Some(rest) = line.strip_prefix("data:") {
                data = Some(rest.trim().to_string());
            }
        }
        if let Some(json) = data {
            if let Ok(env) = serde_json::from_str::<Envelope>(&json) {
                out.push(env);
            }
        }
    }
    out
}

fn payload_kind(payload: &Value) -> Option<&str> {
    payload
        .get("type")
        .and_then(Value::as_str)
        .or_else(|| payload.get("kind").and_then(Value::as_str))
}

fn short_payload(payload: &Value) -> String {
    let s = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());
    if s.len() > 60 {
        format!("{}\u{2026}", &s[..60])
    } else {
        s
    }
}

fn short_time(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|dt| dt.with_timezone(&Utc).format("%H:%M:%S").to_string())
        .unwrap_or_else(|_| {
            rfc3339
                .split('T')
                .nth(1)
                .and_then(|s| s.split('.').next())
                .unwrap_or(rfc3339)
                .to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_parses_two_consecutive_events() {
        let mut buf = String::from(
            "event: bus\ndata: {\"seq\":1,\"topic\":\"t\",\"sender\":\"a\",\"payload\":{\"type\":\"x\"},\"created_at\":\"2026-05-04T12:00:00Z\"}\n\nevent: bus\ndata: {\"seq\":2,\"topic\":\"t\",\"sender\":null,\"payload\":{},\"created_at\":\"2026-05-04T12:00:01Z\"}\n\n",
        );
        let envs = drain_events(&mut buf);
        assert_eq!(envs.len(), 2);
        assert_eq!(envs[0].seq, 1);
        assert_eq!(envs[1].sender, None);
        assert!(buf.is_empty());
    }

    #[test]
    fn drain_keeps_partial_event() {
        let mut buf = String::from("event: bus\ndata: {\"seq\":1");
        let envs = drain_events(&mut buf);
        assert!(envs.is_empty());
        assert!(buf.starts_with("event: bus"));
    }

    #[test]
    fn drain_skips_keepalive_comment() {
        let mut buf = String::from(": keepalive\n\n");
        let envs = drain_events(&mut buf);
        assert!(envs.is_empty());
        assert!(buf.is_empty());
    }

    #[test]
    fn payload_kind_prefers_type_then_kind() {
        let v = serde_json::json!({"type": "x", "kind": "y"});
        assert_eq!(payload_kind(&v), Some("x"));
        let v = serde_json::json!({"kind": "y"});
        assert_eq!(payload_kind(&v), Some("y"));
        let v = serde_json::json!({});
        assert_eq!(payload_kind(&v), None);
    }
}
