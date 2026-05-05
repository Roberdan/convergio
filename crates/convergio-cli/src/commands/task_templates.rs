//! Task category templates with default `evidence_required` arrays
//! (P2-10).
//!
//! When a user creates a task with `cvg task create --template <kind>`,
//! the daemon sees a non-empty `evidence_required` array out of the
//! box, so the evidence gate (ADR-0044) blocks `submitted` until the
//! agent attaches the canonical evidence kinds for that work category.
//! Without this, agents tend to ship tasks with empty evidence
//! contracts, the validator passes them trivially, and the audit
//! chain captures nothing.

use clap::ValueEnum;

/// Category of work a task represents. Drives the default
/// `evidence_required` array.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum TaskTemplate {
    /// New feature or behavior change. Expects code + a test artifact.
    Impl,
    /// Documentation-only change (ADR, README, spec, agent prompt).
    Docs,
    /// Refactor without behavior change. Same proof as Impl plus a
    /// before/after note.
    Refactor,
    /// Test-only addition (new fixture, new e2e, coverage bump).
    Test,
}

impl TaskTemplate {
    /// Return the canonical `evidence_required` slice for this
    /// template. The slice is sorted and de-duplicated.
    pub fn evidence_required(self) -> &'static [&'static str] {
        match self {
            Self::Impl => &["code", "test_output"],
            Self::Docs => &["doc_link"],
            Self::Refactor => &["code", "test_output"],
            Self::Test => &["test_output"],
        }
    }
}

/// Merge a user-supplied `evidence_required` Vec with the template's
/// default. User-supplied values win on conflict; template defaults
/// fill the rest. Result is sorted + de-duplicated for stability.
pub fn resolve_evidence(user: Vec<String>, template: Option<TaskTemplate>) -> Vec<String> {
    let mut out: Vec<String> = user;
    if let Some(t) = template {
        for default in t.evidence_required() {
            if !out.iter().any(|u| u == default) {
                out.push((*default).to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_template_has_at_least_one_evidence_kind() {
        for t in [
            TaskTemplate::Impl,
            TaskTemplate::Docs,
            TaskTemplate::Refactor,
            TaskTemplate::Test,
        ] {
            assert!(
                !t.evidence_required().is_empty(),
                "template {t:?} must require at least one evidence kind"
            );
        }
    }

    #[test]
    fn template_only_yields_its_defaults_sorted() {
        let v = resolve_evidence(vec![], Some(TaskTemplate::Impl));
        assert_eq!(v, vec!["code".to_string(), "test_output".to_string()]);
    }

    #[test]
    fn user_kinds_extend_template_without_duplicates() {
        let v = resolve_evidence(
            vec!["code".to_string(), "adr".to_string()],
            Some(TaskTemplate::Impl),
        );
        assert_eq!(
            v,
            vec![
                "adr".to_string(),
                "code".to_string(),
                "test_output".to_string()
            ]
        );
    }

    #[test]
    fn no_template_passes_user_kinds_through_sorted() {
        let v = resolve_evidence(
            vec![
                "zebra".to_string(),
                "alpha".to_string(),
                "alpha".to_string(),
            ],
            None,
        );
        assert_eq!(v, vec!["alpha".to_string(), "zebra".to_string()]);
    }

    #[test]
    fn empty_user_no_template_yields_empty() {
        assert!(resolve_evidence(vec![], None).is_empty());
    }
}
