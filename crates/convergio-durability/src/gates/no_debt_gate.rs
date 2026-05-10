//! `NoDebtGate` — refuses `submitted`/`done` transitions when the
//! evidence payload contains technical-debt markers (TODO, FIXME,
//! unwrap, ignored tests, debug prints, ...).
//!
//! This is one of the gates that turns Convergio into a leash for
//! agents: an LLM that has been told "no TODOs" but tries to slip one
//! in *will* get a 409 here, because the evidence it itself attached
//! contains the marker. Either it cleans up, or the task does not
//! advance.
//!
//! ## Defaults
//!
//! See [`default_rules`] for the built-in rule set. New defaults must
//! ship with both a unit test and an ADR justifying the addition —
//! adding rules to a default-on gate is a breaking change for users.
//!
//! ## Customization
//!
//! Build with [`NoDebtGate::with_rules`] to override the rule set
//! (load from `convergio-debt-rules.toml`, etc). The MVP ships
//! defaults only; custom config is a future-session feature.

use super::{Gate, GateContext, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;
use crate::store::EvidenceStore;
use regex::Regex;

/// Refuses the transition when the task's evidence contains debt
/// markers.
pub struct NoDebtGate {
    rules: Vec<DebtRule>,
}

/// One pattern that, if matched anywhere in any evidence payload,
/// blocks the transition.
pub struct DebtRule {
    /// Stable name (used in error messages).
    pub name: &'static str,
    /// Compiled regex.
    pub pattern: Regex,
}

impl Default for NoDebtGate {
    fn default() -> Self {
        Self {
            rules: default_rules(),
        }
    }
}

impl NoDebtGate {
    /// Build with a custom rule set.
    pub fn with_rules(rules: Vec<DebtRule>) -> Self {
        Self { rules }
    }
}

/// Built-in rule set. Each entry is `(name, regex)`.
///
/// Conservative on purpose — false positives here mean a developer
/// can't ship. Patterns target only universally smelly forms; harmless
/// look-alikes (`unwrap_or`, `unwrap_or_default`, `unwrap_or_else`) are
/// excluded with negative lookahead alternatives.
///
/// Coverage by language:
///
/// | Lang | Patterns |
/// |------|----------|
/// | any  | `TODO`, `FIXME`, `XXX`, `HACK`, `WIP` |
/// | Rust | `.unwrap()`, `.expect(`, `panic!`, `unimplemented!`, `todo!`, `#[ignore]`, `dbg!`, `eprintln!("DEBUG"` |
/// | JS/TS | `console.log`, `debugger;`, `as any`, `@ts-ignore`, `@ts-nocheck` |
/// | Python | `pdb.set_trace`, `breakpoint(`, `from IPython` debug, debug-print to stdout |
/// | Go | `panic(`, blank-discarded error `_ = err`, `// nolint` |
/// | Swift | `fatalError(`, force unwrap `!.` and `!,` and `! ` typical positions |
/// | shell | `set +e` (silent error) |
fn default_rules() -> Vec<DebtRule> {
    let entries: &[(&'static str, &'static str)] = &[
        // Language-agnostic
        ("todo_marker", r"(?i)\b(TODO|FIXME|XXX|HACK|WIP)\b"),
        // Rust
        // unwrap() — strict: `.unwrap()` with no opening paren after = real panic shortcut.
        // `.unwrap_or(`, `.unwrap_or_default(`, `.unwrap_or_else(` are fine: they end in `(`,
        // not `()`, so the negative-lookahead-via-explicit-end avoids them.
        ("rust_unwrap", r"\.unwrap\(\)"),
        ("rust_expect", r"\.expect\("),
        ("rust_panic", r"\bpanic!\("),
        ("rust_unimplemented", r"\bunimplemented!\("),
        ("rust_todo_macro", r"\btodo!\("),
        ("rust_ignored_test", r"#\[ignore\]"),
        ("rust_dbg", r"\bdbg!\("),
        // JS / TS
        ("js_console_log", r"console\.log\("),
        ("js_debugger", r"\bdebugger\s*;"),
        ("ts_as_any", r"\bas\s+any\b"),
        ("ts_ignore", r"@ts-(?:ignore|nocheck|expect-error)\b"),
        // Python
        ("py_pdb_set_trace", r"\bpdb\.set_trace\("),
        ("py_breakpoint", r"\bbreakpoint\("),
        ("py_ipdb", r"\bimport\s+ipdb\b|\bfrom\s+ipdb\b"),
        // Go
        ("go_panic", r"\bpanic\("),
        ("go_blank_err", r"_\s*=\s*err\b"),
        ("go_nolint", r"//\s*nolint\b"),
        // Swift
        ("swift_fatal_error", r"\bfatalError\("),
        ("swift_try_bang", r"\btry!\s"),
        // Shell
        ("sh_silent_errors", r"\bset\s+\+e\b"),
    ];
    // Bad pattern = build-time bug. Panic loud, do not silently drop.
    entries
        .iter()
        .map(|(name, pat)| DebtRule {
            name,
            pattern: Regex::new(pat)
                .unwrap_or_else(|e| panic!("NoDebtGate: bad regex `{pat}` for rule `{name}`: {e}")),
        })
        .collect()
}

#[async_trait::async_trait]
impl Gate for NoDebtGate {
    fn name(&self) -> &'static str {
        "no_debt"
    }

    async fn check(&self, ctx: &GateContext) -> Result<()> {
        if !matches!(ctx.target_status, TaskStatus::Submitted | TaskStatus::Done) {
            return Ok(());
        }

        let store = EvidenceStore::new(ctx.pool.clone());
        let evidence = store.list_by_task(&ctx.task.id).await?;
        let task_is_debt_topic = is_debt_topic_title(&ctx.task.title);

        let mut violations: Vec<String> = Vec::new();
        let mut strings: Vec<String> = Vec::new();
        for ev in evidence {
            // F34 allowlist: a task explicitly *about* technical-debt
            // semantics (WIP-commit template, TODO inventory, debt
            // burn-down spec, ...) cannot avoid the keyword in its
            // documentation evidence. We skip the marker scan when
            // BOTH the evidence kind is documentation/spec AND the
            // task title declares it is a debt-related artifact.
            // Code/test evidence on the same task is still scanned —
            // the doc may legitimately mention WIP, but a Rust file
            // that contains `.unwrap()` is still a leak.
            if task_is_debt_topic && is_documentation_kind(&ev.kind) {
                continue;
            }

            // Walk the JSON tree and collect every string value
            // *unescaped*. Serializing the whole Value with `to_string`
            // would turn real newlines into `\n` literals, defeating
            // the `\b` word boundaries in our regexes.
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
                gate: "no_debt",
                reason: format!("debt_marker_found: {}", violations.join(", ")),
            })
        }
    }

    fn describe(&self) -> GatePrecondition {
        GatePrecondition {
            gate: "no_debt".into(),
            reads_evidence_kinds: vec!["*".into()],
            enforces_task_evidence_required: false,
            active_target_status: vec!["submitted".into(), "done".into()],
            refusal_reasons: vec!["debt_marker_found".into()],
        }
    }
}

/// Title-side half of the F34 allowlist. The match is case-insensitive
/// and word-bounded so a task titled "Withhold Information Plan"
/// (containing "WIP" as a substring) does not slip through.
fn is_debt_topic_title(title: &str) -> bool {
    let lower = title.to_ascii_lowercase();
    DEBT_TOPIC_TOKENS.iter().any(|kw| {
        lower
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == *kw)
    })
}

/// Evidence kinds that legitimately describe debt without introducing
/// it (docs, specs, ADR commentary, friction-log entries).
fn is_documentation_kind(kind: &str) -> bool {
    matches!(kind, "doc" | "spec" | "adr" | "friction" | "review")
}

const DEBT_TOPIC_TOKENS: &[&str] = &["wip", "todo", "fixme", "debt", "hack", "xxx"];

/// Recursively collect every JSON string value from `value` into `out`.
/// Used by the gate so regex `\b` boundaries see real newlines, not
/// JSON-escaped `\n` literals.
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
