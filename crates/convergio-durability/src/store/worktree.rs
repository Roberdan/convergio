//! Reverse lookup from agent-worktree directory slugs to the task /
//! plan that owns them.
//!
//! The executor's dispatch guard ([`convergio-executor::guards`])
//! caps the number of physical worktrees under
//! `<repo>/.claude/worktrees/`. When that cap trips, the operator
//! needs to know **which** entities are holding those worktrees so
//! they can decide whether to wait, kill, or raise the cap. The
//! filesystem alone only carries the 7-char task-id prefix in the
//! directory name (`agent-<task7>`) — Layer 1 owns the rest.
//!
//! This store does the join: take a list of slugs, return the
//! richest "who's holding this" tuple we can produce from the
//! `tasks` + `plans` tables.

use crate::error::Result;
use chrono::{DateTime, Utc};
use convergio_db::Pool;
use serde::{Deserialize, Serialize};

/// Information about a single agent worktree directory, joined to
/// the task and plan that own it (when discoverable).
///
/// `slug` is always set — it's whatever directory name the executor
/// observed on disk. Every other field is `Option<…>` because the
/// underlying task or plan may have been deleted out-of-band, or
/// the worktree may have been left behind by a previous binary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorktreeHolder {
    /// Directory basename without `agent-` prefix (the 7-char task id slug).
    pub slug: String,
    /// Owning task id when a `tasks.id LIKE '<slug>%'` match was found.
    pub task_id: Option<String>,
    /// Task status (`pending`, `in_progress`, `submitted`, ...).
    pub task_status: Option<String>,
    /// Owning plan id.
    pub plan_id: Option<String>,
    /// Project-scoped plan number rendered as `#N` in operator output.
    pub plan_number: Option<i64>,
    /// First-in-progress timestamp (NULL until the task was claimed).
    pub started_at: Option<DateTime<Utc>>,
    /// Agent id that currently holds the claim, when one is recorded.
    pub agent_id: Option<String>,
}

/// Read-only joins between the on-disk `.claude/worktrees/` layout
/// and the Layer 1 task/plan tables.
#[derive(Clone)]
pub struct WorktreeStore {
    pool: Pool,
}

impl WorktreeStore {
    /// Wrap a pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Resolve each slug to its owning task/plan (when one exists).
    ///
    /// Slugs that do not match any task id prefix are returned with
    /// every `Option<…>` set to `None`; the executor uses that as
    /// the "orphan worktree" signal in the refusal message.
    ///
    /// We deliberately match with `tasks.id LIKE '<slug>%'`: the
    /// directory name carries only the first 7 chars of the task
    /// uuid (see `convergio_executor::worktree::worktree_path`).
    /// A 7-char prefix is plenty unique for the dispatch caps
    /// (default 2 worktrees) — if two tasks ever shared a slug
    /// we'd pick one deterministically and the operator would see
    /// duplicates in the listing.
    pub async fn holders_for_slugs(&self, slugs: &[&str]) -> Result<Vec<WorktreeHolder>> {
        let mut out = Vec::with_capacity(slugs.len());
        for slug in slugs {
            out.push(self.holder_for_slug(slug).await?);
        }
        Ok(out)
    }

    async fn holder_for_slug(&self, slug: &str) -> Result<WorktreeHolder> {
        // Slugs are 7-char hex UUID prefixes. `%` or `_` in a slug
        // would become LIKE wildcards and match unrelated tasks.
        if slug.is_empty() || !slug.chars().all(|c| c.is_ascii_hexdigit()) {
            return Ok(orphan_holder(slug));
        }
        let like = format!("{slug}%");
        let row = sqlx::query_as::<_, HolderRow>(
            "SELECT t.id AS task_id, t.status, t.plan_id, t.started_at, t.agent_id, \
             p.number AS plan_number \
             FROM tasks t \
             LEFT JOIN plans p ON p.id = t.plan_id \
             WHERE t.id LIKE ? \
             ORDER BY \
                 CASE t.status \
                     WHEN 'in_progress' THEN 0 \
                     WHEN 'submitted'   THEN 1 \
                     WHEN 'pending'     THEN 2 \
                     ELSE 3 END ASC, \
                 t.updated_at DESC \
             LIMIT 1",
        )
        .bind(&like)
        .fetch_optional(self.pool.inner())
        .await?;

        Ok(match row {
            Some(r) => WorktreeHolder {
                slug: slug.to_string(),
                task_id: Some(r.task_id),
                task_status: Some(r.status),
                plan_id: Some(r.plan_id),
                plan_number: r.plan_number,
                started_at: r.started_at.as_deref().and_then(parse_ts),
                agent_id: r.agent_id,
            },
            None => orphan_holder(slug),
        })
    }
}

fn orphan_holder(slug: &str) -> WorktreeHolder {
    WorktreeHolder {
        slug: slug.to_string(),
        task_id: None,
        task_status: None,
        plan_id: None,
        plan_number: None,
        started_at: None,
        agent_id: None,
    }
}

#[derive(sqlx::FromRow)]
struct HolderRow {
    task_id: String,
    status: String,
    plan_id: String,
    started_at: Option<String>,
    agent_id: Option<String>,
    plan_number: Option<i64>,
}

fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}
