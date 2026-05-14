//! File-backed action log for the MCP bridge.
//!
//! Extracted from `bridge.rs` (audit finding L7) so the bridge file
//! can focus on tool declarations and the rmcp wiring while file
//! IO, log rotation, and the HOME-resolution warn live next to each
//! other and can be unit-tested independently.

use convergio_api::{Action, AgentResponse};
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_BYTES: u64 = 256 * 1024;
const TAIL_LINES: usize = 1000;

/// Append a single JSON line describing the action and its outcome.
///
/// Failures to resolve the log path are surfaced through `tracing::warn`
/// (no payload data) so MCP diagnostics aren't silently lost when HOME
/// is unset or non-unicode. All other IO failures are best-effort —
/// the bridge must never block an action on log persistence.
pub(crate) fn append(action: Action, response: &AgentResponse) {
    let path = match mcp_log_path() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "mcp log path unavailable; skipping action log");
            return;
        }
    };
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    trim_log(&path);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default();
    let line = json!({
        "ts": ts,
        "action": action.as_str(),
        "ok": response.ok,
        "code": response.code,
        "next": response.next,
    });
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = writeln!(file, "{line}");
    }
}

fn mcp_log_path() -> Result<PathBuf, std::env::VarError> {
    Ok(PathBuf::from(std::env::var("HOME")?).join(".convergio/mcp.log"))
}

fn trim_log(path: &Path) {
    let Ok(meta) = fs::metadata(path) else {
        return;
    };
    if meta.len() <= MAX_LOG_BYTES {
        return;
    }
    if let Ok(content) = fs::read_to_string(path) {
        let keep: Vec<&str> = content.lines().rev().take(TAIL_LINES).collect();
        let trimmed = keep.into_iter().rev().collect::<Vec<_>>().join("\n");
        let _ = fs::write(path, format!("{trimmed}\n"));
    }
}
