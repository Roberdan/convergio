//! Storage-side helpers for [`super::cluster`].
//!
//! Kept in a sibling module so that `cluster.rs` stays under the
//! 300-line per-file cap (CONSTITUTION § 13).

use crate::error::Result;
use crate::store::Store;
use sqlx::Row;
use std::collections::BTreeMap;

/// Distinct source-file paths that have at least one node belonging
/// to `crate_name`.
pub(super) async fn files_for_crate(store: &Store, crate_name: &str) -> Result<Vec<String>> {
    let rows = sqlx::query(
        "SELECT DISTINCT file_path FROM graph_nodes \
         WHERE crate_name = ? AND file_path IS NOT NULL \
         ORDER BY file_path",
    )
    .bind(crate_name)
    .fetch_all(store.pool().inner())
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let p: Option<String> = row.try_get("file_path")?;
        if let Some(p) = p {
            out.push(p);
        }
    }
    Ok(out)
}

/// Per-file `item` count for `crate_name`.
pub(super) async fn item_counts_per_file(
    store: &Store,
    crate_name: &str,
) -> Result<BTreeMap<String, u32>> {
    let rows = sqlx::query(
        "SELECT file_path, COUNT(*) AS c FROM graph_nodes \
         WHERE crate_name = ? AND kind = 'item' AND file_path IS NOT NULL \
         GROUP BY file_path",
    )
    .bind(crate_name)
    .fetch_all(store.pool().inner())
    .await?;
    let mut out = BTreeMap::new();
    for row in rows {
        let p: Option<String> = row.try_get("file_path")?;
        let c: i64 = row.try_get("c")?;
        if let Some(p) = p {
            out.insert(p, c as u32);
        }
    }
    Ok(out)
}

/// Returns weighted file→file edges aggregated from item-level `uses`
/// edges that stay inside the named crate. Self-loops (item used in
/// the same file) are dropped, and undirected pairs collapse to a
/// canonical (lex-min, lex-max) key.
pub(super) async fn file_uses_edges(
    store: &Store,
    crate_name: &str,
) -> Result<BTreeMap<(String, String), u32>> {
    let rows = sqlx::query(
        "SELECT src_n.file_path AS src_file, dst_n.file_path AS dst_file, \
                SUM(e.weight) AS w \
         FROM graph_edges e \
         JOIN graph_nodes src_n ON e.src = src_n.id \
         JOIN graph_nodes dst_n ON e.dst = dst_n.id \
         WHERE e.kind = 'uses' \
           AND src_n.crate_name = ? AND dst_n.crate_name = ? \
           AND src_n.file_path IS NOT NULL AND dst_n.file_path IS NOT NULL \
           AND src_n.file_path != dst_n.file_path \
         GROUP BY src_n.file_path, dst_n.file_path",
    )
    .bind(crate_name)
    .bind(crate_name)
    .fetch_all(store.pool().inner())
    .await?;
    let mut out = BTreeMap::new();
    for row in rows {
        let s: Option<String> = row.try_get("src_file")?;
        let d: Option<String> = row.try_get("dst_file")?;
        let w: i64 = row.try_get("w")?;
        if let (Some(s), Some(d)) = (s, d) {
            let key = if s < d { (s, d) } else { (d, s) };
            *out.entry(key).or_insert(0) += w as u32;
        }
    }
    Ok(out)
}

/// Approximate LOC of a file via `read_to_string().lines().count()`.
/// Returns `Err` on I/O failure so callers can distinguish a
/// genuinely empty file from a deleted or unreadable one (audit
/// 2026-05 follow-up: silent `0` hid disk-state bugs).
pub(super) fn file_loc(path: &str) -> Result<u64> {
    let s = std::fs::read_to_string(path).map_err(crate::error::GraphError::Io)?;
    Ok(s.lines().count() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Regression: silently returning `0` for unreadable files makes
    /// "file deleted" and "file empty" indistinguishable. The audit
    /// 2026-05 follow-up requires `file_loc` to surface the I/O
    /// failure so callers can log + skip explicitly.
    #[test]
    fn file_loc_distinguishes_missing_from_empty() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist.rs");
        let err = file_loc(missing.to_str().unwrap()).expect_err("must surface I/O error");
        assert!(matches!(err, crate::error::GraphError::Io(_)));
    }

    #[test]
    fn file_loc_counts_lines_for_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("x.rs");
        std::fs::write(&p, "a\nb\nc\n").unwrap();
        assert_eq!(file_loc(p.to_str().unwrap()).unwrap(), 3);
    }
}
