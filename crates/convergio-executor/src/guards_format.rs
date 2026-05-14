//! Refusal-message formatting helpers extracted from
//! `convergio_executor::guards`. Pulled into a sibling module so
//! `guards.rs` stays under the 300-line cap and so the formatting
//! can be unit-tested independently of the on-disk guard.

use convergio_durability::WorktreeHolder;

/// Format one holder into the short identity blob used in the
/// refusal message.
pub(crate) fn render_holder(h: &WorktreeHolder) -> String {
    let head = format!("agent-{}", h.slug);
    let mut inner: Vec<String> = Vec::new();
    if let Some(n) = h.plan_number {
        inner.push(format!("plan #{n}"));
    } else if let Some(pid) = h.plan_id.as_deref() {
        inner.push(format!("plan {}", pid.get(..7).unwrap_or(pid)));
    }
    if let Some(tid) = h.task_id.as_deref() {
        inner.push(format!("task {}", tid.get(..7).unwrap_or(tid)));
    }
    if let Some(status) = h.task_status.as_deref() {
        inner.push(format!("status={status}"));
    }
    if let Some(started) = h.started_at {
        inner.push(format!("claimed {}", human_age(started)));
    } else if h.task_id.is_none() {
        inner.push("orphan (no matching task)".to_string());
    }
    if inner.is_empty() {
        head
    } else {
        format!("{} ({})", head, inner.join(", "))
    }
}

/// Format every holder for the in-use list of a guard refusal.
pub(crate) fn render_holders(holders: &[WorktreeHolder]) -> String {
    holders
        .iter()
        .map(render_holder)
        .collect::<Vec<_>>()
        .join("; ")
}

fn human_age(ts: chrono::DateTime<chrono::Utc>) -> String {
    let now = chrono::Utc::now();
    let delta = now.signed_duration_since(ts);
    let secs = delta.num_seconds().max(0);
    if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86_400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86_400)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_holder_with_full_metadata() {
        let h = WorktreeHolder {
            slug: "abc1234".into(),
            task_id: Some("abc1234-5678-90ab-cdef-000000000000".into()),
            task_status: Some("in_progress".into()),
            plan_id: Some("11111111-2222-3333-4444-555555555555".into()),
            plan_number: Some(5),
            started_at: Some(chrono::Utc::now() - chrono::Duration::hours(2)),
            agent_id: Some("copilot-abc1234".into()),
        };
        let rendered = render_holder(&h);
        assert!(rendered.starts_with("agent-abc1234 ("), "got: {rendered}");
        assert!(rendered.contains("plan #5"), "got: {rendered}");
        assert!(rendered.contains("task abc1234"), "got: {rendered}");
        assert!(rendered.contains("status=in_progress"), "got: {rendered}");
        assert!(rendered.contains("claimed 2h ago"), "got: {rendered}");
    }

    #[test]
    fn render_holder_orphan_is_marked() {
        let h = WorktreeHolder {
            slug: "deadbee".into(),
            task_id: None,
            task_status: None,
            plan_id: None,
            plan_number: None,
            started_at: None,
            agent_id: None,
        };
        let rendered = render_holder(&h);
        assert!(rendered.contains("orphan"), "got: {rendered}");
    }

    #[test]
    fn render_holder_falls_back_to_plan_id_prefix_without_number() {
        let h = WorktreeHolder {
            slug: "abc1234".into(),
            task_id: Some("abc12340000".into()),
            task_status: Some("pending".into()),
            plan_id: Some("11111111-2222-3333".into()),
            plan_number: None,
            started_at: None,
            agent_id: None,
        };
        let rendered = render_holder(&h);
        assert!(rendered.contains("plan 1111111"), "got: {rendered}");
    }

    #[test]
    fn human_age_buckets() {
        let now = chrono::Utc::now();
        assert!(human_age(now - chrono::Duration::seconds(10)).ends_with("s ago"));
        assert!(human_age(now - chrono::Duration::minutes(5)).ends_with("m ago"));
        assert!(human_age(now - chrono::Duration::hours(3)).ends_with("h ago"));
        assert!(human_age(now - chrono::Duration::days(2)).ends_with("d ago"));
    }
}
