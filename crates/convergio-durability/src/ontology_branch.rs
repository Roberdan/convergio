//! Ontology branch domain types and overlay store.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Lifecycle of an ontology branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OntologyBranchStatus {
    /// Branch exists but is not ready for merge.
    Draft,
    /// Branch is under review.
    Review,
    /// Branch has been merged into mainline.
    Merged,
    /// Branch has been discarded.
    Discarded,
}

impl OntologyBranchStatus {
    /// String tag persisted in the DB.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Review => "review",
            Self::Merged => "merged",
            Self::Discarded => "discarded",
        }
    }

    /// Parse from the DB.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "draft" => Some(Self::Draft),
            "review" => Some(Self::Review),
            "merged" => Some(Self::Merged),
            "discarded" => Some(Self::Discarded),
            _ => None,
        }
    }

    /// Whether writes are allowed against this status.
    pub fn is_open(&self) -> bool {
        matches!(self, Self::Draft | Self::Review)
    }
}

/// A persistent ontology branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyBranch {
    /// UUID v4.
    pub id: String,
    /// Stable branch name.
    pub name: String,
    /// Current status.
    pub status: OntologyBranchStatus,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last-update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Timestamp when moved into review.
    #[serde(default)]
    pub reviewed_at: Option<DateTime<Utc>>,
    /// Timestamp when merged.
    #[serde(default)]
    pub merged_at: Option<DateTime<Utc>>,
    /// Timestamp when discarded.
    #[serde(default)]
    pub discarded_at: Option<DateTime<Utc>>,
}

/// Materialized ontology entry resolved for a given read context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyResolvedEntry {
    /// Logical key.
    pub key: String,
    /// Resolved JSON value, or `null` when absent.
    pub value: Value,
    /// Source of the resolved value.
    pub source: OntologyValueSource,
}

/// Where a resolved entry value came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OntologyValueSource {
    /// Branch overlay value.
    Branch,
    /// Mainline value.
    Main,
    /// No value present.
    None,
}
