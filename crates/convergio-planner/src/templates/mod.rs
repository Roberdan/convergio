//! Plan template registry (W6, ADR-0056).
//!
//! A *plan template* is a parametric scaffold: a title + objective +
//! list of task shapes with `{{var}}` placeholders that the operator
//! fills in at render time. Rendering yields a [`PlanShape`] that any
//! existing planner downstream consumer can persist.
//!
//! Templates are first-party Rust today (one constant per template).
//! A follow-up will add a `--template-file <path>` loader for
//! capability-supplied YAML/JSON files; the in-memory shape is the
//! same.

use crate::error::{PlannerError, Result};
use crate::schema::{PlanShape, TaskShape};
use std::collections::HashMap;

/// One parameter the operator must supply at render time.
#[derive(Debug, Clone)]
pub struct TemplateParam {
    /// Short snake-case key. Used in `{{var}}` substitutions.
    pub name: &'static str,
    /// One-line description shown by `cvg plan-templates show`.
    pub help: &'static str,
}

/// A parametric task scaffold.
#[derive(Debug, Clone)]
pub struct TemplateTask {
    /// Wave number (1-indexed).
    pub wave: i64,
    /// Sequence within the wave (1-indexed).
    pub sequence: i64,
    /// Title — may contain `{{var}}` placeholders.
    pub title: &'static str,
    /// Optional description — may contain `{{var}}` placeholders.
    pub description: Option<&'static str>,
    /// Evidence kinds the task must attach. Static — no substitution.
    pub evidence_required: &'static [&'static str],
}

/// A parametric plan scaffold.
#[derive(Debug, Clone)]
pub struct Template {
    /// Kebab-case identifier.
    pub name: &'static str,
    /// One-line operator description.
    pub summary: &'static str,
    /// Long-form description — may contain `{{var}}` placeholders.
    pub description: Option<&'static str>,
    /// Objective statement (stored on the plan via PlanCoherenceGate,
    /// W4). May contain `{{var}}` placeholders.
    pub objective: &'static str,
    /// Required parameters.
    pub parameters: &'static [TemplateParam],
    /// Title for the rendered plan — may contain `{{var}}`.
    pub title: &'static str,
    /// Tasks the template emits.
    pub tasks: &'static [TemplateTask],
}

impl Template {
    /// Render the template into a [`PlanShape`] suitable for the
    /// existing planner persistence path. Returns
    /// `PlannerError::Template` when a required parameter is missing or
    /// when an unknown placeholder is referenced.
    pub fn render(&self, params: &HashMap<String, String>) -> Result<RenderedTemplate> {
        for p in self.parameters {
            if !params.contains_key(p.name) {
                return Err(PlannerError::Template(format!(
                    "missing parameter `{}` for template `{}`",
                    p.name, self.name
                )));
            }
        }
        let title = substitute(self.title, params, self.name)?;
        let description = match self.description {
            Some(d) => Some(substitute(d, params, self.name)?),
            None => None,
        };
        let objective = substitute(self.objective, params, self.name)?;
        let mut tasks = Vec::with_capacity(self.tasks.len());
        for t in self.tasks {
            tasks.push(TaskShape {
                wave: t.wave,
                sequence: t.sequence,
                title: substitute(t.title, params, self.name)?,
                description: match t.description {
                    Some(d) => Some(substitute(d, params, self.name)?),
                    None => None,
                },
                evidence_required: t
                    .evidence_required
                    .iter()
                    .map(|s| (*s).to_string())
                    .collect(),
                runner_kind: None,
                profile: None,
                max_budget_usd: None,
            });
        }
        Ok(RenderedTemplate {
            objective,
            plan: PlanShape {
                title,
                description,
                tasks,
            },
        })
    }
}

/// Output of [`Template::render`]: the plan shape plus the objective
/// that the caller should write via the W4 plan-objectives store.
#[derive(Debug, Clone)]
pub struct RenderedTemplate {
    /// Objective to persist on the plan after creation.
    pub objective: String,
    /// Plan + task shapes ready to persist.
    pub plan: PlanShape,
}

fn substitute(text: &str, params: &HashMap<String, String>, template_name: &str) -> Result<String> {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("}}").ok_or_else(|| {
            PlannerError::Template(format!(
                "unterminated `{{{{` placeholder in template `{template_name}`"
            ))
        })?;
        let key = after[..end].trim();
        let value = params.get(key).ok_or_else(|| {
            PlannerError::Template(format!(
                "unknown placeholder `{{{{{key}}}}}` in template `{template_name}`"
            ))
        })?;
        out.push_str(value);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

mod dsar_gdpr_v1;
mod incident_response_gdpr_v1;
mod vertical_accelerator_v1;

pub use dsar_gdpr_v1::DSAR_GDPR_V1;
pub use incident_response_gdpr_v1::INCIDENT_RESPONSE_GDPR_V1;
pub use vertical_accelerator_v1::VERTICAL_ACCELERATOR_V1;

/// All first-party templates known to this build.
pub const BUILTIN: &[&Template] = &[
    &VERTICAL_ACCELERATOR_V1,
    &DSAR_GDPR_V1,
    &INCIDENT_RESPONSE_GDPR_V1,
];

/// List every built-in template.
pub fn list_builtin() -> &'static [&'static Template] {
    BUILTIN
}

/// Look up a built-in template by name.
pub fn get_builtin(name: &str) -> Option<&'static Template> {
    BUILTIN.iter().copied().find(|t| t.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn lists_builtin_templates() {
        let names: Vec<_> = list_builtin().iter().map(|t| t.name).collect();
        for expected in [
            "vertical-accelerator-v1",
            "dsar-gdpr-v1",
            "incident-response-gdpr-v1",
        ] {
            assert!(names.contains(&expected), "missing template: {expected}");
        }
    }

    #[test]
    fn renders_vertical_accelerator_with_all_parameters() {
        let p = params(&[
            ("domain", "education"),
            ("primary_language", "en"),
            ("secondary_language", "it"),
            ("target_audience", "K-12 teachers"),
        ]);
        let r = VERTICAL_ACCELERATOR_V1.render(&p).unwrap();
        assert_eq!(r.plan.title, "education vertical accelerator");
        assert!(r.objective.contains("education"));
        assert!(r.objective.contains("K-12 teachers"));
        assert_eq!(r.plan.tasks.len(), 5);
        assert!(r.plan.tasks[0].title.contains("K-12 teachers"));
        assert!(r.plan.tasks[2].description.as_ref().unwrap().contains("en"));
    }

    #[test]
    fn refuses_missing_parameter() {
        let p = params(&[("domain", "education")]); // others missing
        let err = VERTICAL_ACCELERATOR_V1.render(&p).unwrap_err();
        assert!(
            format!("{err}").contains("missing parameter"),
            "unexpected err: {err}"
        );
    }

    #[test]
    fn refuses_unknown_placeholder() {
        let bad = Template {
            name: "bad",
            summary: "",
            description: None,
            objective: "hi {{nope}}",
            parameters: &[],
            title: "t",
            tasks: &[],
        };
        let err = bad.render(&HashMap::new()).unwrap_err();
        assert!(format!("{err}").contains("unknown placeholder"));
    }

    #[test]
    fn substitute_passes_through_text_without_placeholders() {
        let p = HashMap::new();
        let out = substitute("plain text", &p, "x").unwrap();
        assert_eq!(out, "plain text");
    }
}
