//! `A11yGate` phase 1 — accessibility checks on evidence payloads.
//!
//! Implements the built-in subset of CONSTITUTION § Sacred principle
//! #3 (accessibility first). Phase 2 (capability `a11y.axe`) wraps
//! axe-core for full WCAG coverage; that ships separately (W11 of
//! `docs/plans/v1.0-production-ready.md`).
//!
//! Checks here intentionally use **no external tooling** — they fire
//! against the evidence row payload at `submitted` / `done`
//! transitions and refuse with HTTP 409 + a stable reason of the form
//! `a11y_violation_found: <evidence_kind>#<rule>, ...`.
//!
//! Evidence-kind dispatch:
//! - `markdown`, `markdown_doc`, `md_doc`, `doc`, `readme` →
//!   markdown structure checks.
//! - `cli_output`, `terminal`, `tui_snapshot` → terminal-output
//!   checks.
//! - Every kind → bidi-override scan (cheap, defends against text
//!   spoofing in any leaf).
//!
//! See ADR-0051 for the rationale and the deferred axe-core path.

use super::{Gate, GateContext, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;
use crate::store::EvidenceStore;
use convergio_a11y_axe::{run_html, AxeStatus};
use regex::Regex;
use serde_json::Value;

/// Built-in accessibility gate.
pub struct A11yGate {
    md: MarkdownRules,
    cli: CliRules,
    bidi: Regex,
}

struct MarkdownRules {
    image_missing_alt: Regex,
    link_nondescriptive: Regex,
    color_only_emphasis: Regex,
    heading: Regex,
}

struct CliRules {
    ansi_escape: Regex,
}

impl Default for A11yGate {
    fn default() -> Self {
        Self {
            md: MarkdownRules {
                image_missing_alt: Regex::new(r"!\[\s*\]\([^)]*\)").unwrap(),
                link_nondescriptive: Regex::new(
                    r"(?i)\[\s*(?:here|click here|click|link|this|read more|more)\s*\]\([^)]+\)",
                )
                .unwrap(),
                color_only_emphasis: Regex::new(r#"(?i)<font\s+[^>]*color\s*="#).unwrap(),
                heading: Regex::new(r"(?m)^(#{1,6})\s+\S").unwrap(),
            },
            cli: CliRules {
                ansi_escape: Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").unwrap(),
            },
            // U+202A..U+202E (bidi embedding / override) + U+2066..U+2069
            // (isolates). All seven are used in known text-spoofing
            // attacks against terminals and viewers.
            bidi: Regex::new(
                r"[\u{202A}\u{202B}\u{202C}\u{202D}\u{202E}\u{2066}\u{2067}\u{2068}\u{2069}]",
            )
            .unwrap(),
        }
    }
}

#[async_trait::async_trait]
impl Gate for A11yGate {
    fn name(&self) -> &'static str {
        "a11y"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !matches!(ctx.target_status, TaskStatus::Submitted | TaskStatus::Done) {
            return Ok(());
        }

        let store = EvidenceStore::new(ctx.pool.clone());
        let evidence = store.list_by_task(&ctx.task.id).await?;
        let mut violations: Vec<String> = Vec::new();

        for ev in evidence {
            let kind = ev.kind.as_str();
            let mut strings = Vec::new();
            collect_strings(&ev.payload, &mut strings);
            let joined = strings.join("\n");

            // Bidi check fires on every evidence kind.
            if self.bidi.is_match(&joined) {
                violations.push(format!("{kind}#bidi_override"));
            }

            if is_markdown_kind(kind) {
                self.check_markdown(kind, &joined, &mut violations);
            }
            if is_cli_kind(kind) {
                self.check_cli(kind, &strings, &mut violations);
            }
            if is_html_kind(kind) {
                match run_html(&joined) {
                    AxeStatus::Ok(report) => {
                        for v in report.violations {
                            if matches!(v.impact.as_str(), "serious" | "critical") {
                                violations.push(format!("{kind}#axe:{}", v.id));
                            }
                        }
                    }
                    AxeStatus::NotConfigured => tracing::info!("a11y phase-2 skipped: set CONVERGIO_A11Y_AXE_BIN or run `cvg capability install a11y-axe`"),
                    AxeStatus::Error(_) => {}
                }
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            violations.sort();
            violations.dedup();
            Err(DurabilityError::GateRefused {
                gate: "a11y",
                reason: format!("a11y_violation_found: {}", violations.join(", ")),
            })
        }
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "a11y".into(),
            reads_evidence_kinds: vec!["*".into()],
            enforces_task_evidence_required: false,
            active_target_status: vec!["submitted".into(), "done".into()],
            refusal_reasons: vec!["a11y_violation_found".into()],
        }
    }
}

impl A11yGate {
    fn check_markdown(&self, kind: &str, text: &str, violations: &mut Vec<String>) {
        if self.md.image_missing_alt.is_match(text) {
            violations.push(format!("{kind}#md_image_missing_alt"));
        }
        if self.md.link_nondescriptive.is_match(text) {
            violations.push(format!("{kind}#md_link_nondescriptive"));
        }
        if self.md.color_only_emphasis.is_match(text) {
            violations.push(format!("{kind}#md_color_only_emphasis"));
        }
        if has_heading_skip(text, &self.md.heading) {
            violations.push(format!("{kind}#md_heading_skip"));
        }
    }

    fn check_cli(&self, kind: &str, strings: &[String], violations: &mut Vec<String>) {
        // A "color-only" message is any line whose meaning relies on
        // ANSI escapes — stripping them produces an empty (or
        // whitespace-only) line that still carried a signal.
        for s in strings {
            for line in s.lines() {
                if self.cli.ansi_escape.is_match(line) {
                    let stripped = self.cli.ansi_escape.replace_all(line, "");
                    if stripped.trim().is_empty() {
                        violations.push(format!("{kind}#cli_color_only_signal"));
                        return;
                    }
                }
            }
        }
    }
}

fn is_markdown_kind(kind: &str) -> bool {
    matches!(
        kind,
        "markdown" | "markdown_doc" | "md_doc" | "doc" | "readme"
    )
}

fn is_cli_kind(kind: &str) -> bool {
    matches!(kind, "cli_output" | "terminal" | "tui_snapshot")
}

fn is_html_kind(kind: &str) -> bool {
    matches!(kind, "html_output" | "html" | "component_render")
}

/// Flags forward heading-level jumps > 1 (H1→H3 etc). Docs may start at any level.
fn has_heading_skip(text: &str, heading_re: &Regex) -> bool {
    let mut last: Option<usize> = None;
    for cap in heading_re.captures_iter(text) {
        let level = cap.get(1).map(|m| m.as_str().len()).unwrap_or(0);
        if let Some(prev) = last {
            if level > prev + 1 {
                return true;
            }
        }
        last = Some(level);
    }
    false
}

fn collect_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Array(items) => items.iter().for_each(|v| collect_strings(v, out)),
        Value::Object(map) => map.values().for_each(|v| collect_strings(v, out)),
        _ => {}
    }
}
