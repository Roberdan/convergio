//! Fleet configuration schema — mirrors `~/.convergio/v3/fleet.toml`
//! (ADR-0038 § 5.6).

use serde::{Deserialize, Serialize};

/// Top-level fleet configuration file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetConfig {
    /// Global fleet identity and retrieval knobs.
    pub fleet: FleetSection,

    /// Retrieval tuning (optional, falls back to defaults).
    #[serde(default)]
    pub retrieval: RetrievalSection,

    /// Registered repositories.
    #[serde(rename = "repo", default)]
    pub repos: Vec<RepoEntry>,
}

/// `[fleet]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetSection {
    /// Human-readable fleet name.
    pub name: String,

    /// Default branch used for graph builds.
    #[serde(default = "default_branch")]
    pub default_branch: String,
}

fn default_branch() -> String {
    "main".to_owned()
}

/// `[retrieval]` section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalSection {
    /// Linear-blend weight for structural vs semantic (0.0 = pure
    /// semantic, 1.0 = pure structural). Default 0.5.
    #[serde(default = "default_alpha")]
    pub alpha: f64,

    /// Embedding model identifier (e.g. "bge-m3-small-int8").
    #[serde(default = "default_embed_model")]
    pub embed_model: String,

    /// Number of top results returned by fleet retrieval.
    #[serde(default = "default_top_k")]
    pub top_k: usize,
}

impl Default for RetrievalSection {
    fn default() -> Self {
        Self {
            alpha: default_alpha(),
            embed_model: default_embed_model(),
            top_k: default_top_k(),
        }
    }
}

fn default_alpha() -> f64 {
    0.5
}
fn default_embed_model() -> String {
    "bge-m3-small-int8".to_owned()
}
fn default_top_k() -> usize {
    25
}

/// A single `[[repo]]` entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoEntry {
    /// Short slug that uniquely identifies this repo in the fleet.
    pub name: String,

    /// Absolute path on disk.
    pub path: String,

    /// Primary language (e.g. "rust", "typescript", "python").
    pub language: String,

    /// Parser backend ("syn" for Rust, "tree-sitter" for others).
    pub parser: String,

    /// Role of this repo in the fleet.
    #[serde(default = "default_role")]
    pub role: RepoRole,

    /// Optional parent repo this one derives from.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derives_from: Option<String>,
}

/// Role of a repo within the fleet. Informs retrieval weighting and
/// dead-code thresholds (ADR-0038 § 5.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RepoRole {
    /// The Convergio engine repo.
    Engine,
    /// A shared library consumed by downstream repos.
    Library,
    /// A repo that depends on the engine or library.
    #[default]
    Downstream,
    /// An experimental or prototype repo.
    Sandbox,
}

impl RepoRole {
    /// Return the canonical string representation stored in the DB.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Engine => "engine",
            Self::Library => "library",
            Self::Downstream => "downstream",
            Self::Sandbox => "sandbox",
        }
    }
}

impl std::fmt::Display for RepoRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RepoRole {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "engine" => Ok(Self::Engine),
            "library" => Ok(Self::Library),
            "downstream" => Ok(Self::Downstream),
            "sandbox" => Ok(Self::Sandbox),
            other => Err(format!("unknown repo role: {other}")),
        }
    }
}

fn default_role() -> RepoRole {
    RepoRole::Downstream
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[fleet]
name = "test-fleet"
default_branch = "main"

[retrieval]
alpha = 0.5
embed_model = "bge-m3-small-int8"
top_k = 25

[[repo]]
name = "convergio"
path = "/repo/convergio"
language = "rust"
parser = "syn"
role = "engine"

[[repo]]
name = "convergio-edu"
path = "/repo/convergio-edu"
language = "typescript"
parser = "tree-sitter"
role = "downstream"
derives_from = "convergio"
"#;

    #[test]
    fn parse_sample_config() {
        let cfg: FleetConfig = toml::from_str(SAMPLE).expect("parse");
        assert_eq!(cfg.fleet.name, "test-fleet");
        assert_eq!(cfg.retrieval.alpha, 0.5);
        assert_eq!(cfg.repos.len(), 2);
        assert_eq!(cfg.repos[0].role, RepoRole::Engine);
        assert_eq!(cfg.repos[1].derives_from.as_deref(), Some("convergio"));
    }

    #[test]
    fn roundtrip_config() {
        let cfg: FleetConfig = toml::from_str(SAMPLE).expect("parse");
        let serialized = toml::to_string(&cfg).expect("serialize");
        let cfg2: FleetConfig = toml::from_str(&serialized).expect("re-parse");
        assert_eq!(cfg2.fleet.name, cfg.fleet.name);
        assert_eq!(cfg2.repos.len(), cfg.repos.len());
    }

    #[test]
    fn repo_role_roundtrip() {
        for (s, expected) in [
            ("engine", RepoRole::Engine),
            ("library", RepoRole::Library),
            ("downstream", RepoRole::Downstream),
            ("sandbox", RepoRole::Sandbox),
        ] {
            let r: RepoRole = s.parse().expect("parse");
            assert_eq!(r, expected);
            assert_eq!(r.as_str(), s);
        }
    }

    #[test]
    fn defaults_apply_when_retrieval_absent() {
        let minimal = r#"
[fleet]
name = "minimal"

[[repo]]
name = "foo"
path = "/foo"
language = "rust"
parser = "syn"
"#;
        let cfg: FleetConfig = toml::from_str(minimal).expect("parse");
        assert_eq!(cfg.fleet.default_branch, "main");
        assert_eq!(cfg.retrieval.alpha, 0.5);
        assert_eq!(cfg.retrieval.top_k, 25);
        assert_eq!(cfg.repos[0].role, RepoRole::Downstream);
    }
}
