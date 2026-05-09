//! `WireCheckGate` — refuses `submitted`/`done` when the agent
//! claims to have wired routes or CLI paths that do not actually
//! exist in the workspace.
//!
//! Sister gate to [`super::NoStubGate`]. Where `NoStubGate` is a
//! regex-only scan over evidence payloads and admits in its own
//! doc-comment that it cannot catch *"the agent claiming a route
//! is mounted when it isn't"*, this gate closes that gap by
//! reading a structured `wire_check` evidence row and verifying
//! every claimed entity actually exists in the workspace tree.
//!
//! ## Evidence shape
//!
//! Agents attach evidence rows of `kind == "wire_check"` whose
//! `payload` is JSON of shape:
//!
//! ```json
//! {
//!   "routes":    [{"method": "GET", "path": "/v1/agent-registry/agents"}],
//!   "cli_paths": ["agent list", "plan list"]
//! }
//! ```
//!
//! Both keys are optional; an empty / missing payload is a silent
//! pass. This gate is **opt-in by design**: agents that do not
//! attach a `wire_check` row are not refused. CONSTITUTION P4
//! discipline + a future `ClaimCheckGate` (F55-B) is responsible
//! for forcing structured wiring claims.
//!
//! ## Heuristic limits
//!
//! The route check is a substring scan for `.route("PATH"` text
//! across `crates/convergio-server/src/routes/**/*.rs`. It cannot
//! detect:
//!
//! - A route declared but never `.merge()`d into the top-level
//!   `Router` (the file exists, the literal exists, but the router
//!   wiring is missing).
//! - A route added inside a `cfg(test)` block.
//!
//! The CLI check is a heuristic substring scan: for the claim
//! `"<top> <sub>"` we confirm
//! `crates/convergio-cli/src/commands/<top>.rs` exists and contains
//! the `<sub>` token (case-insensitive). It cannot detect:
//!
//! - A subcommand variant declared but never matched in the
//!   dispatch `match` arm (the variant exists, the dispatch is
//!   missing).
//! - A subcommand whose impl lives in a sibling helper module
//!   (e.g. `<top>_render.rs`) — but every shipped subcommand keeps
//!   the variant name in `<top>.rs`, which is the file we scan.
//!
//! Closing those structural gaps is the responsibility of the
//! follow-up `ClaimCheckGate` (F55-B). See
//! `crates/convergio-durability/src/gates/no_stub_gate.rs:7-21`
//! for the parent gate's original list of "things regex cannot
//! catch" — this gate handles the second bullet (route claim).
//!
//! ## Environment
//!
//! Workspace root comes from `CONVERGIO_WIRE_CHECK_ROOT`, falling
//! back to `std::env::current_dir()`. If the resolved path does
//! not contain a `crates/` directory, the gate **silently passes**
//! — refusing on a missing workspace would block transitions for
//! every operator whose cwd is not the repo root, including CI
//! environments running the daemon from a packaged binary.

use super::{Gate, GateContext, GateEvidenceInput, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;
use crate::store::EvidenceStore;
use serde::Deserialize;
use std::path::{Path, PathBuf};

mod scan;

/// Refuses when an agent's `wire_check` evidence claims routes or
/// CLI paths that cannot be located in the workspace.
pub struct WireCheckGate;

/// Parsed `wire_check` evidence payload.
#[derive(Debug, Default, Deserialize)]
struct WireCheckPayload {
    #[serde(default)]
    routes: Vec<RouteClaim>,
    #[serde(default)]
    cli_paths: Vec<String>,
}

/// One claimed HTTP route.
#[derive(Debug, Deserialize)]
struct RouteClaim {
    /// HTTP verb (informational; included in refusal messages but
    /// not used for matching — axum routers register a path once
    /// and chain verbs via `.route(path, get(..).post(..))`).
    method: String,
    /// URL path, e.g. `/v1/agent-registry/agents`.
    path: String,
}

#[async_trait::async_trait]
impl Gate for WireCheckGate {
    fn name(&self) -> &'static str {
        "wire_check"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !matches!(ctx.target_status, TaskStatus::Submitted | TaskStatus::Done) {
            return Ok(());
        }

        let root = match resolve_root() {
            Some(r) => r,
            None => return Ok(()),
        };

        let store = EvidenceStore::new(ctx.pool.clone());
        let evidence = store.list_by_task(&ctx.task.id).await?;

        let mut payloads: Vec<WireCheckPayload> = Vec::new();
        for ev in evidence {
            if ev.kind != "wire_check" {
                continue;
            }
            // Tolerate a malformed payload by treating it as empty;
            // structural validation is not this gate's job.
            if let Ok(p) = serde_json::from_value::<WireCheckPayload>(ev.payload) {
                payloads.push(p);
            }
        }
        if payloads.is_empty() {
            return Ok(());
        }

        let routes_root = root.join("crates/convergio-server/src/routes");
        let cli_commands_root = root.join("crates/convergio-cli/src/commands");
        let route_haystack = scan::collect_route_text(&routes_root);

        let mut missing: Vec<String> = Vec::new();
        for payload in &payloads {
            for claim in &payload.routes {
                if !scan::route_is_mounted(&route_haystack, &claim.path) {
                    missing.push(format!(
                        "route not mounted: {} {}",
                        claim.method, claim.path
                    ));
                }
            }
            for cli in &payload.cli_paths {
                if !scan::cli_path_exists(&cli_commands_root, cli) {
                    missing.push(format!("cli path not found: {cli}"));
                }
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            missing.sort();
            missing.dedup();
            Err(DurabilityError::GateRefused {
                gate: "wire_check",
                reason: missing.join("; "),
            })
        }
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "wire_check",
            evidence: GateEvidenceInput::SpecificKinds {
                kinds: vec!["wire_check"],
            },
            active_target_status: vec!["submitted", "done"],
            refusal_reasons: vec!["route_not_mounted", "cli_path_not_found"],
        }
    }
}

/// Resolve the workspace root the gate scans against.
///
/// Returns `None` when the resolved path does not exist or does not
/// contain a `crates/` directory — in that case the gate silently
/// passes (see module docs for rationale).
fn resolve_root() -> Option<PathBuf> {
    let raw = std::env::var("CONVERGIO_WIRE_CHECK_ROOT")
        .ok()
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())?;
    if !is_workspace(&raw) {
        return None;
    }
    Some(raw)
}

/// Heuristic: a directory is a Convergio workspace root iff it
/// contains a `crates/` subdirectory. Cheap and correct for our
/// monorepo layout.
fn is_workspace(path: &Path) -> bool {
    path.join("crates").is_dir()
}
