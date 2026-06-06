//! The authoring request: what the operator asks the tool to model.
//!
//! Per the product vision, input is deliberately flexible: the operator
//! may supply **documents** (standards, regulations, specs), a free-form
//! **intent** (a prompt + industry + use-case), or both. At least one of
//! the two must be present.

use std::path::PathBuf;

use crate::error::{AuthorError, Result};

/// A free-form description of what to model when (or in addition to)
/// providing source documents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Intent {
    /// Natural-language description of the desired ontology.
    pub prompt: String,
    /// Target industry / domain (e.g. `higher-education`).
    pub industry: String,
    /// The concrete use-case (e.g. `student-information-system`).
    pub use_case: String,
}

impl Intent {
    /// `true` when every field is blank — i.e. no usable intent.
    pub fn is_blank(&self) -> bool {
        self.prompt.trim().is_empty()
            && self.industry.trim().is_empty()
            && self.use_case.trim().is_empty()
    }
}

/// A complete authoring request.
#[derive(Debug, Clone, Default)]
pub struct AuthoringRequest {
    /// Optional generic intent.
    pub intent: Option<Intent>,
    /// Optional source documents to ground the ontology in.
    pub documents: Vec<PathBuf>,
}

impl AuthoringRequest {
    /// Build a request from an intent alone.
    pub fn from_intent(intent: Intent) -> Self {
        Self {
            intent: Some(intent),
            documents: Vec::new(),
        }
    }

    /// Validate that the request carries usable input, returning an
    /// error when it is effectively empty.
    pub fn ensure_non_empty(&self) -> Result<()> {
        let has_intent = self.intent.as_ref().is_some_and(|i| !i.is_blank());
        if has_intent || !self.documents.is_empty() {
            Ok(())
        } else {
            Err(AuthorError::EmptyRequest)
        }
    }
}
