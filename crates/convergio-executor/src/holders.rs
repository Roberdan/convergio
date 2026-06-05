//! Worktree-holder enrichment: list the on-disk worktree slugs,
//! ask Layer 1 to resolve them to tasks/plans, return the joined
//! vec for guard refusal messages.
//!
//! Pulled out of `executor.rs` so that module stays under the
//! 300-line cap.

use convergio_durability::{Durability, WorktreeHolder};
use std::path::Path;

/// Read `.claude/worktrees/` and resolve each agent slug against
/// Layer 1. Errors from the DB lookup are logged and downgraded to
/// an empty vec — the guard must still trip even if the join
/// cannot be performed.
pub(crate) async fn collect(durability: &Durability, repo_root: &Path) -> Vec<WorktreeHolder> {
    let slugs = crate::guards::list_worktree_slugs(repo_root);
    if slugs.is_empty() {
        return Vec::new();
    }
    let refs: Vec<&str> = slugs.iter().map(String::as_str).collect();
    match durability.worktrees().holders_for_slugs(&refs).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                error = %e,
                "worktree holder lookup failed; refusal message will lack detail"
            );
            Vec::new()
        }
    }
}

/// Count active workspace leases for dispatch pressure accounting.
pub(crate) async fn active_lease_count(durability: &Durability) -> usize {
    match durability.workspace().active_leases().await {
        Ok(v) => v.len(),
        Err(e) => {
            tracing::warn!(error = %e, "workspace lease count failed; assuming zero");
            0
        }
    }
}
