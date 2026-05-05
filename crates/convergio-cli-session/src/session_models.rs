//! Shared data shapes for `cvg session resume`.
//!
//! Split out of `session.rs` to keep both files under the 300-line
//! cap (CONSTITUTION § 13). The structs are `pub(crate)` because the
//! sibling `render` module borrows them.

use serde::{Deserialize, Serialize};

/// One row from `GET /v1/plans` / `GET /v1/plans/:id`.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Plan {
    pub(crate) id: String,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) project: Option<String>,
    pub(crate) status: String,
    pub(crate) updated_at: String,
}

/// One row from `GET /v1/plans/:id/tasks`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Task {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) status: String,
    pub(crate) wave: i64,
    pub(crate) sequence: i64,
    pub(crate) created_at: String,
}

/// Bucketed task counts for the cold-start brief.
#[derive(Debug, Default, Serialize)]
pub(crate) struct TaskCounts {
    pub(crate) total: usize,
    pub(crate) done: usize,
    pub(crate) pending: usize,
    pub(crate) in_progress: usize,
    pub(crate) submitted: usize,
    pub(crate) failed: usize,
}

impl From<&[Task]> for TaskCounts {
    fn from(tasks: &[Task]) -> Self {
        let mut c = TaskCounts {
            total: tasks.len(),
            ..Default::default()
        };
        for t in tasks {
            match t.status.as_str() {
                "done" => c.done += 1,
                "pending" => c.pending += 1,
                "in_progress" => c.in_progress += 1,
                "submitted" => c.submitted += 1,
                "failed" => c.failed += 1,
                _ => {}
            }
        }
        c
    }
}

/// Trimmed `gh pr list` JSON row.
#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct PrSummary {
    pub(crate) number: i64,
    pub(crate) title: String,
    #[serde(rename = "headRefName")]
    pub(crate) head_ref_name: String,
    #[serde(rename = "isDraft", default)]
    pub(crate) is_draft: bool,
}
