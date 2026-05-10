//! `NoStubGate` — refuses `submitted`/`done` when evidence contains
//! explicit scaffolding markers.
//!
//! Sister gate to [`super::NoDebtGate`], dedicated to **P4 — no
//! scaffolding only**. Where `NoDebtGate` catches "I left a TODO",
//! this catches the more specific failure mode "I created a file
//! but didn't wire it" — agents tend to leave a comment that gives
//! it away (`// stub`, `// to be wired`, `// placeholder`, etc.).
//!
//! ## Limits of the regex approach
//!
//! Regex catches **declared** stubs. It cannot catch:
//!
//! - The agent silently leaving a function unused (no caller).
//! - The agent claiming a route is mounted when it isn't.
//! - The agent inventing a test name that does not exist.
//!
//! Those require diff parsing and a structured `wire_check` evidence
//! kind. Planned in `WireCheckGate` and `ClaimCheckGate`. This gate
//! handles the polite-stub case where the agent at least *admits*
//! the work is incomplete.

use super::{Gate, GateContext, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;
use crate::store::EvidenceStore;
use regex::Regex;

/// Refuses on stub markers in evidence payloads.
pub struct NoStubGate {
    rules: Vec<StubRule>,
}

/// One stub-marker pattern.
pub struct StubRule {
    /// Stable name (used in error messages).
    pub name: &'static str,
    /// Compiled regex.
    pub pattern: Regex,
}

impl Default for NoStubGate {
    fn default() -> Self {
        Self {
            rules: default_rules(),
        }
    }
}

impl NoStubGate {
    /// Build with a custom rule set.
    pub fn with_rules(rules: Vec<StubRule>) -> Self {
        Self { rules }
    }
}

/// Default scaffolding-marker rule set.
///
/// All comment-prefix patterns use the alternation
/// `(?://|\#|--|<!--|/\*)`, so the rule fires regardless of the
/// language's comment syntax:
///
/// | Family | Comment marker |
/// |--------|----------------|
/// | C / C++ / Rust / Go / Java / Kotlin / Swift / JS / TS | `//` |
/// | Python / Ruby / Bash / YAML / TOML / Conf | `#` |
/// | SQL / Lua / Haskell | `--` |
/// | HTML / XML / Markdown | `<!--` |
/// | CSS / Block-comments in C-family | `/*` |
///
/// Scaffolding markers caught:
///
/// | Marker | Why it fires |
/// |--------|--------------|
/// | `<comment> stub` / `stubbed` | Explicit "I'm leaving a stub" |
/// | `<comment> scaffolding` / `scaffold` | Explicit scaffolding admission |
/// | `<comment> placeholder` | Explicit placeholder admission |
/// | `<comment> to be (wired\|connected\|implemented\|done\|finished)` | Self-acknowledged TODO |
/// | `<comment> not (yet )?(wired\|connected\|implemented\|hooked( up)?\|in use)` | Negative form |
/// | `(skeleton)` parenthetical anywhere | Common marker for skeleton files |
/// | `unreachable!(` (Rust) / `raise NotImplementedError` (Python) / `throw new NotImplementedException` (Java/C#/TS) / `throw new UnsupportedOperationException` (Java) | Language-idiomatic "I'll get to this" |
fn default_rules() -> Vec<StubRule> {
    // Comment prefix shared by every comment-marker rule. Matches
    // the start of a line (with optional whitespace) followed by any
    // common comment marker. The trailing `\s*` lets us write the
    // keyword without worrying about exact whitespace.
    // Comment-prefix alternation. Note Rust's `regex` rejects `\#`
    // as an unknown escape, so `#` must appear unescaped (it has no
    // metacharacter meaning anyway). No lookaround: shebang `#!` may
    // be a false-positive source, but nobody writes `#! placeholder`
    // for real.
    const C: &str = r"(?im)(?:^|[\r\n])\s*(?://|#|--|<!--|/\*)\s*";

    let entries: &[(&'static str, String)] = &[
        ("stub_comment", format!(r"{C}stub(?:bed)?\b")),
        ("scaffold_comment", format!(r"{C}scaffold(?:ing)?\b")),
        ("placeholder_comment", format!(r"{C}placeholder\b")),
        (
            "to_be_done",
            format!(r"{C}to\s+be\s+(?:wired|connected|implemented|done|finished|completed)\b"),
        ),
        (
            "not_wired",
            format!(
                r"{C}not\s+(?:yet\s+)?(?:wired|connected|implemented|hooked(?:\s+up)?|in\s+use)\b"
            ),
        ),
        // Language-agnostic marker (anywhere, not only in comment).
        ("skeleton_marker", r"(?i)\(\s*skeleton\s*\)".to_string()),
        // Language-idiomatic "I'll get to this":
        ("rust_unreachable", r"\bunreachable!\(".to_string()),
        (
            "py_not_implemented",
            r"\braise\s+NotImplementedError\b".to_string(),
        ),
        (
            "jvm_not_implemented",
            r"\bthrow\s+new\s+NotImplementedException\b".to_string(),
        ),
        (
            "jvm_unsupported_op",
            r"\bthrow\s+new\s+UnsupportedOperationException\b".to_string(),
        ),
    ];
    // Compile every pattern up front. A bad regex in this list is a
    // build-time bug, not a silent feature loss — we panic.
    entries
        .iter()
        .map(|(name, pat)| StubRule {
            name,
            pattern: Regex::new(pat)
                .unwrap_or_else(|e| panic!("NoStubGate: bad regex `{pat}` for rule `{name}`: {e}")),
        })
        .collect()
}

#[async_trait::async_trait]
impl Gate for NoStubGate {
    fn name(&self) -> &'static str {
        "no_stub"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !matches!(ctx.target_status, TaskStatus::Submitted | TaskStatus::Done) {
            return Ok(());
        }

        let store = EvidenceStore::new(ctx.pool.clone());
        let evidence = store.list_by_task(&ctx.task.id).await?;

        let mut violations: Vec<String> = Vec::new();
        let mut strings: Vec<String> = Vec::new();
        for ev in evidence {
            strings.clear();
            collect_strings(&ev.payload, &mut strings);
            for s in &strings {
                for rule in &self.rules {
                    if rule.pattern.is_match(s) {
                        violations.push(format!("{}#{}", ev.kind, rule.name));
                    }
                }
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            violations.sort();
            violations.dedup();
            Err(DurabilityError::GateRefused {
                gate: "no_stub",
                reason: format!("scaffolding_marker_found: {}", violations.join(", ")),
            })
        }
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "no_stub".into(),
            reads_evidence_kinds: vec!["*".into()],
            enforces_task_evidence_required: false,
            active_target_status: vec!["submitted".into(), "done".into()],
            refusal_reasons: vec!["scaffolding_marker_found".into()],
        }
    }
}

/// Recursive helper — same shape as `no_debt_gate::collect_strings`.
fn collect_strings(value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(items) => {
            for item in items {
                collect_strings(item, out);
            }
        }
        serde_json::Value::Object(map) => {
            for (_k, v) in map {
                collect_strings(v, out);
            }
        }
        _ => {}
    }
}
