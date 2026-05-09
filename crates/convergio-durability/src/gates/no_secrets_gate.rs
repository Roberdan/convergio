//! `NoSecretsGate` — refuses evidence that appears to contain secrets.

use super::{Gate, GateContext, GateEvidenceInput, GatePrecondition};
use crate::error::{DurabilityError, Result};
use crate::model::TaskStatus;
use crate::store::EvidenceStore;
use regex::Regex;

/// Refuses common credential leaks in evidence payloads.
pub struct NoSecretsGate {
    rules: Vec<SecretRule>,
}

/// One secret pattern.
pub struct SecretRule {
    /// Stable name for refusal messages.
    pub name: &'static str,
    /// Compiled regex.
    pub pattern: Regex,
}

impl Default for NoSecretsGate {
    fn default() -> Self {
        Self {
            rules: default_rules(),
        }
    }
}

impl NoSecretsGate {
    /// Build with custom secret rules.
    pub fn with_rules(rules: Vec<SecretRule>) -> Self {
        Self { rules }
    }
}

fn default_rules() -> Vec<SecretRule> {
    let entries: &[(&str, &str)] = &[
        ("private_key", r"-----BEGIN [A-Z ]*PRIVATE KEY-----"),
        ("aws_access_key", r"\bAKIA[0-9A-Z]{16}\b"),
        (
            "github_token",
            r"\b(?:ghp|gho|ghu|ghs|ghr)_[A-Za-z0-9_]{36,}\b",
        ),
        (
            "github_pat",
            r"\bgithub_pat_[A-Za-z0-9_]{22}_[A-Za-z0-9_]{59}\b",
        ),
        ("slack_token", r"\bxox[baprs]-[A-Za-z0-9-]{20,}\b"),
        ("stripe_live_key", r"\bsk_live_[A-Za-z0-9]{16,}\b"),
    ];
    entries
        .iter()
        .map(|(name, pat)| SecretRule {
            name,
            pattern: Regex::new(pat).unwrap_or_else(|e| {
                panic!("NoSecretsGate: bad regex `{pat}` for rule `{name}`: {e}")
            }),
        })
        .collect()
}

#[async_trait::async_trait]
impl Gate for NoSecretsGate {
    fn name(&self) -> &'static str {
        "no_secrets"
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
                gate: "no_secrets",
                reason: format!(
                    "secret-like values found in evidence: {}",
                    violations.join(", ")
                ),
            })
        }
    }

    fn describe(&self) -> GatePrecondition {
        let mut rules: Vec<&'static str> = self.rules.iter().map(|r| r.name).collect();
        rules.sort();
        rules.dedup();
        GatePrecondition {
            gate: "no_secrets",
            evidence: GateEvidenceInput::AllKinds,
            active_target_status: vec!["submitted", "done"],
            refusal_reasons: rules,
        }
    }
}

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
