//! Runner registry — config-driven vendor specs (ADR-0035).
//!
//! Loads vendor declarations from `~/.convergio/runners.toml` so
//! adding a new vendor (qwen, codex, gemini) does not require a
//! Convergio recompile. Each spec describes how to translate a
//! [`SpawnContext`] + a model name into a concrete vendor-CLI argv.
//!
//! The two reference vendors (`claude`, `copilot`) keep their
//! hardcoded runners — the registry is checked only for vendors
//! the [`Family`] enum does not recognise.
//!
//! [`SpawnContext`]: crate::SpawnContext
//! [`Family`]: crate::Family

use crate::error::{Result, RunnerError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// How the prompt is delivered to the vendor CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PromptVia {
    /// Pipe the prompt on stdin (preferred for long prompts).
    #[default]
    Stdin,
    /// Pass the prompt as the value of `prompt_argv_flag` (default
    /// `-p`). Used by CLIs that do not yet read stdin.
    Argv,
}

/// Per-permission-profile flag set. Mapped by lowercase tag —
/// `standard`, `read_only`, `sandbox` (matches
/// [`PermissionProfile::tag`]).
///
/// [`PermissionProfile::tag`]: crate::PermissionProfile::tag
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProfileSpec {
    /// Argv fragments to inject for this profile.
    #[serde(default)]
    pub args: Vec<String>,
}

/// Spec for one custom vendor. Mirrors the shape of the entries in
/// `~/.convergio/runners.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerSpec {
    /// Binary name on `PATH` (e.g. `qwen`, `gemini`).
    pub cli: String,
    /// Where the prompt goes (stdin or argv).
    #[serde(default)]
    pub prompt_via: PromptVia,
    /// When `prompt_via = "argv"`, the flag that prefixes the prompt
    /// (default `-p`). Ignored for stdin.
    #[serde(default = "default_prompt_argv_flag")]
    pub prompt_argv_flag: String,
    /// Flag that selects the model (e.g. `--model`).
    #[serde(default = "default_model_flag")]
    pub model_flag: String,
    /// Always-on extra args (e.g. `["--no-stream"]`).
    #[serde(default)]
    pub extra_args: Vec<String>,
    /// Optional model allowlist. When empty, any model is accepted —
    /// new models surface without a registry edit.
    #[serde(default)]
    pub models: Vec<String>,
    /// Per-profile flag sets. Keys are profile tags
    /// (`standard` / `read_only` / `sandbox`).
    #[serde(default)]
    pub profiles: HashMap<String, ProfileSpec>,
}

fn default_prompt_argv_flag() -> String {
    "-p".into()
}

fn default_model_flag() -> String {
    "--model".into()
}

/// Top-level shape of `~/.convergio/runners.toml`. Vendors are keyed
/// by their wire-format `vendor` tag (matches [`RunnerKind::vendor`]).
///
/// [`RunnerKind::vendor`]: crate::RunnerKind::vendor
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunnerRegistry {
    /// Map keyed by vendor name — `[vendors.qwen] cli = "qwen" ...`.
    #[serde(default)]
    pub vendors: HashMap<String, RunnerSpec>,
}

impl RunnerRegistry {
    /// Empty registry — used when no `runners.toml` is present and
    /// in unit tests that only exercise built-in vendors.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Look up a vendor by name. Custom vendors only — the caller is
    /// expected to short-circuit built-ins via [`Family::from_tag`].
    ///
    /// [`Family::from_tag`]: crate::Family::from_tag
    pub fn get(&self, vendor: &str) -> Option<&RunnerSpec> {
        self.vendors.get(vendor)
    }

    /// Parse a TOML string. Used by [`Self::load`] and by tests.
    pub fn parse(src: &str) -> Result<Self> {
        toml::from_str(src).map_err(|e| RunnerError::RegistryInvalid(e.to_string()))
    }

    /// Load from `path`. Returns an empty registry when the file
    /// does not exist (the common case on a fresh install).
    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(src) => Self::parse(&src),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::empty()),
            Err(e) => Err(RunnerError::Io(e)),
        }
    }

    /// Resolve and load the default registry path.
    ///
    /// Order: `$CONVERGIO_RUNNERS_TOML` (test override) → `$HOME/.convergio/runners.toml`.
    /// Missing file → empty registry (the default install).
    pub fn load_default() -> Result<Self> {
        let path = default_path();
        Self::load_from(&path)
    }
}

/// The path the daemon reads the registry from. Allows
/// `$CONVERGIO_RUNNERS_TOML` to override for tests; otherwise
/// `$HOME/.convergio/runners.toml`.
pub fn default_path() -> PathBuf {
    if let Some(p) = std::env::var_os("CONVERGIO_RUNNERS_TOML") {
        return PathBuf::from(p);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join(".convergio").join("runners.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    const QWEN_TOML: &str = r#"
[vendors.qwen]
cli = "qwen"
prompt_via = "stdin"
model_flag = "--model"
extra_args = ["--no-stream"]
models = ["qwen-coder", "qwen-max"]

[vendors.qwen.profiles.standard]
args = ["--read-only=false"]

[vendors.qwen.profiles.read_only]
args = ["--read-only=true"]

[vendors.qwen.profiles.sandbox]
args = ["--allow-all"]
"#;

    #[test]
    fn parses_qwen_spec() {
        let r = RunnerRegistry::parse(QWEN_TOML).unwrap();
        let q = r.get("qwen").expect("qwen present");
        assert_eq!(q.cli, "qwen");
        assert_eq!(q.prompt_via, PromptVia::Stdin);
        assert_eq!(q.model_flag, "--model");
        assert_eq!(q.extra_args, vec!["--no-stream"]);
        assert_eq!(q.models.len(), 2);
        assert_eq!(
            q.profiles.get("read_only").unwrap().args,
            vec!["--read-only=true"]
        );
    }

    #[test]
    fn empty_registry_has_no_vendors() {
        let r = RunnerRegistry::empty();
        assert!(r.get("qwen").is_none());
    }

    #[test]
    fn missing_file_yields_empty_registry() {
        let r = RunnerRegistry::load_from(Path::new("/nonexistent/runners.toml")).unwrap();
        assert!(r.get("qwen").is_none());
    }

    #[test]
    fn invalid_toml_is_rejected() {
        let err = RunnerRegistry::parse("not = valid = toml").unwrap_err();
        assert!(matches!(err, RunnerError::RegistryInvalid(_)));
    }

    #[test]
    fn defaults_apply_when_optional_fields_omitted() {
        let r = RunnerRegistry::parse(
            r#"
[vendors.tiny]
cli = "tiny"
"#,
        )
        .unwrap();
        let t = r.get("tiny").unwrap();
        assert_eq!(t.prompt_via, PromptVia::Stdin);
        assert_eq!(t.model_flag, "--model");
        assert_eq!(t.prompt_argv_flag, "-p");
        assert!(t.extra_args.is_empty());
        assert!(t.profiles.is_empty());
    }
}
