//! Plan-level task counts.
//!
//! Split out of [`crate::client`] so that file stays under the
//! 300-line cap. Same shape as before — only the location changed.

use crate::client::TaskSummary;

/// Aggregated task counts per status, used by the Plans pane.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlanCounts {
    /// Total tasks observed.
    pub total: usize,
    /// `done` count.
    pub done: usize,
    /// `pending` count.
    pub pending: usize,
    /// `in_progress` count.
    pub in_progress: usize,
    /// `submitted` count.
    pub submitted: usize,
    /// `failed` count.
    pub failed: usize,
}

impl PlanCounts {
    /// Build from a list of tasks.
    pub fn from_tasks(tasks: &[TaskSummary]) -> Self {
        let mut c = PlanCounts {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn t(status: &str) -> TaskSummary {
        TaskSummary {
            id: "x".into(),
            plan_id: "p".into(),
            title: "t".into(),
            status: status.into(),
            agent_id: None,
            created_at: "2026-05-02T20:11:00Z".into(),
            updated_at: "2026-05-02T20:11:00Z".into(),
            ..TaskSummary::default()
        }
    }

    #[test]
    fn plan_counts_groups_statuses() {
        let xs = [
            t("done"),
            t("done"),
            t("pending"),
            t("in_progress"),
            t("submitted"),
            t("failed"),
        ];
        let c = PlanCounts::from_tasks(&xs);
        assert_eq!(c.total, 6);
        assert_eq!(
            (c.done, c.pending, c.in_progress, c.submitted, c.failed),
            (2, 1, 1, 1, 1)
        );
    }
}
