//! Graph node search helpers.
//!
//! This is a lightweight, substring-based search over `graph_nodes`.
//! It is meant to power a unified exploration surface (e.g. `/api/search`)
//! without requiring the caller to run the full `for_task` context-pack
//! query.

use crate::error::Result;
use crate::store::Store;
use sqlx::Row;

/// One `graph_nodes` match returned by [`search_nodes`].
#[derive(Debug, Clone)]
pub struct NodeSearchHit {
    /// Stable node id.
    pub id: String,
    /// Node kind (`crate` | `module` | `item` | ...).
    pub kind: String,
    /// Display name.
    pub name: String,
    /// Owning crate.
    pub crate_name: String,
    /// Optional file path (relative to repo root).
    pub file_path: Option<String>,
}

/// Substring search over `graph_nodes`.
///
/// Matches `name`, `id`, or `file_path` case-insensitively. The result set
/// is capped by `limit` (hard-capped at 200).
pub async fn search_nodes(store: &Store, query: &str, limit: usize) -> Result<Vec<NodeSearchHit>> {
    let q = query.trim();
    if q.is_empty() || limit == 0 {
        return Ok(Vec::new());
    }
    let limit = limit.min(200) as i64;

    let escaped = escape_like(q);
    let pattern = format!("%{escaped}%");

    // Keep ordering stable and mildly relevance-biased without trying to be
    // clever: prefer exact-ish name matches, then crates, then modules.
    let rows = sqlx::query(
        "SELECT id, kind, name, crate_name, file_path\n         FROM graph_nodes\n         WHERE (name LIKE ? ESCAPE '\\' COLLATE NOCASE)\n            OR (id LIKE ? ESCAPE '\\' COLLATE NOCASE)\n            OR (file_path LIKE ? ESCAPE '\\' COLLATE NOCASE)\n         ORDER BY\n            CASE WHEN name LIKE ? ESCAPE '\\' COLLATE NOCASE THEN 0 ELSE 1 END,\n            CASE kind WHEN 'crate' THEN 0 WHEN 'module' THEN 1 ELSE 2 END,\n            name ASC, id ASC\n         LIMIT ?",
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .bind(&pattern)
    .bind(limit)
    .fetch_all(store.pool().inner())
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(NodeSearchHit {
            id: row.try_get("id")?,
            kind: row.try_get("kind")?,
            name: row.try_get("name")?,
            crate_name: row.try_get("crate_name")?,
            file_path: row.try_get("file_path")?,
        });
    }
    Ok(out)
}

fn escape_like(value: &str) -> String {
    // SQLite LIKE supports an ESCAPE char; we use '\\'.
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_like_escapes_special_chars() {
        assert_eq!(escape_like("a%b_c"), "a\\%b\\_c");
        assert_eq!(escape_like("a\\b"), "a\\\\b");
    }
}
