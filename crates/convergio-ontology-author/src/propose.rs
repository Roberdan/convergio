//! The proposer seam: turn a prompt into raw ontology JSON.
//!
//! Per ADR-0032 Convergio never calls a raw LLM HTTP API; it shells out
//! to the operator's already-authenticated vendor CLI. [`CliProposer`]
//! does exactly that — binary + args are configurable, the prompt is
//! piped over **stdin** (never argv), and stdout/stderr are captured
//! separately so failures surface cleanly. Tests use [`StubProposer`].

use std::io::Write;
use std::process::{Command, Stdio};

use crate::error::{AuthorError, Result};

/// Produces ontology JSON from a fully-composed prompt.
pub trait OntologyProposer {
    /// A short identifier for provenance (e.g. `claude:cli`).
    fn model_id(&self) -> String;
    /// Run the prompt and return the raw model output.
    fn propose(&self, prompt: &str) -> Result<String>;
}

/// Shells out to a vendor CLI (default `claude -p`), prompt via stdin.
pub struct CliProposer {
    /// CLI binary to invoke.
    pub bin: String,
    /// Extra arguments passed before stdin is read.
    pub args: Vec<String>,
}

impl Default for CliProposer {
    fn default() -> Self {
        Self {
            bin: "claude".to_string(),
            args: vec!["-p".to_string()],
        }
    }
}

impl OntologyProposer for CliProposer {
    fn model_id(&self) -> String {
        format!("cli:{}", self.bin)
    }

    fn propose(&self, prompt: &str) -> Result<String> {
        let mut child = Command::new(&self.bin)
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| AuthorError::Proposer(format!("could not run '{}': {e}", self.bin)))?;

        child
            .stdin
            .take()
            .ok_or_else(|| AuthorError::Proposer("child stdin unavailable".into()))?
            .write_all(prompt.as_bytes())
            .map_err(|e| AuthorError::Proposer(format!("failed to write prompt: {e}")))?;

        let out = child.wait_with_output().map_err(|e| {
            AuthorError::Proposer(format!("failed to wait for '{}': {e}", self.bin))
        })?;
        if !out.status.success() {
            return Err(AuthorError::Proposer(format!(
                "'{}' exited with {}: {}",
                self.bin,
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use std::cell::RefCell;

    /// Deterministic proposer for tests: returns queued responses in
    /// order, so a fail-then-succeed repair path can be exercised.
    pub struct StubProposer {
        responses: RefCell<Vec<String>>,
        /// Prompts received, captured for assertions.
        pub seen: RefCell<Vec<String>>,
    }

    impl StubProposer {
        /// Build a stub that yields `responses` in order (front first).
        pub fn new(responses: Vec<String>) -> Self {
            Self {
                responses: RefCell::new(responses),
                seen: RefCell::new(Vec::new()),
            }
        }
    }

    impl OntologyProposer for StubProposer {
        fn model_id(&self) -> String {
            "stub:test".to_string()
        }

        fn propose(&self, prompt: &str) -> Result<String> {
            self.seen.borrow_mut().push(prompt.to_string());
            let mut r = self.responses.borrow_mut();
            if r.is_empty() {
                Err(AuthorError::Proposer("stub exhausted".into()))
            } else {
                Ok(r.remove(0))
            }
        }
    }
}
