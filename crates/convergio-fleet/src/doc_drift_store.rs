//! SQL helpers + cosine math for [`super::doc_drift`].
//!
//! Pulled out of `doc_drift.rs` purely to keep that file under the
//! 300-line cap; not intended for external use.

use crate::error::Result;
use crate::store::FleetStore;
use convergio_embed::EmbedStore;
use sqlx::Row;
use std::collections::HashMap;

pub(super) struct DocMeta {
    pub repo: String,
    pub name: String,
    pub kind: String,
    pub file_path: Option<String>,
}

pub(super) struct SnapRow {
    pub node_id: String,
    pub snapshot_score: f32,
    pub snapshot_at: String,
}

pub(super) async fn load_embeddings(
    embed: &EmbedStore,
    model: &str,
) -> Result<HashMap<(String, String), Vec<f32>>> {
    Ok(embed
        .all_for_model(model)
        .await?
        .into_iter()
        .map(|(r, n, v)| ((r, n), v))
        .collect())
}

pub(super) async fn load_doc_meta(fleet: &FleetStore) -> Result<HashMap<String, DocMeta>> {
    let rows = match sqlx::query(
        "SELECT id, repo, name, kind, file_path FROM graph_nodes WHERE kind IN ('adr', 'doc')",
    )
    .fetch_all(fleet.pool().inner())
    .await
    {
        Ok(r) => r,
        Err(sqlx::Error::Database(ref e)) if e.message().contains("no such table") => {
            return Ok(HashMap::new())
        }
        Err(e) => return Err(crate::error::FleetError::Db(e)),
    };
    Ok(rows
        .into_iter()
        .map(|row| {
            let id: String = row.get("id");
            (
                id,
                DocMeta {
                    repo: row.get("repo"),
                    name: row.get("name"),
                    kind: row.get("kind"),
                    file_path: row.get("file_path"),
                },
            )
        })
        .collect())
}

pub(super) async fn load_doc_links(
    fleet: &FleetStore,
) -> Result<HashMap<String, Vec<(String, String)>>> {
    let rows = match sqlx::query(
        "SELECT e.src AS src, e.dst AS dst, n.repo AS repo \
         FROM graph_edges e JOIN graph_nodes n ON n.id = e.dst \
         WHERE e.kind IN ('claims', 'mentions')",
    )
    .fetch_all(fleet.pool().inner())
    .await
    {
        Ok(r) => r,
        Err(sqlx::Error::Database(ref e)) if e.message().contains("no such table") => {
            return Ok(HashMap::new())
        }
        Err(e) => return Err(crate::error::FleetError::Db(e)),
    };
    let mut out: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for row in rows {
        let src: String = row.get("src");
        let dst: String = row.get("dst");
        let repo: String = row.get("repo");
        out.entry(src).or_default().push((repo, dst));
    }
    Ok(out)
}

pub(super) async fn load_snapshots(
    fleet: &FleetStore,
    model: &str,
    repo: Option<&str>,
) -> Result<Vec<SnapRow>> {
    let q = if let Some(r) = repo {
        sqlx::query(
            "SELECT s.node_id AS node_id, s.snapshot_score AS s, s.snapshot_at AS at \
             FROM fleet_doc_snapshots s JOIN graph_nodes n ON n.id = s.node_id \
             WHERE s.model = ? AND n.repo = ?",
        )
        .bind(model)
        .bind(r)
    } else {
        sqlx::query(
            "SELECT node_id, snapshot_score AS s, snapshot_at AS at \
             FROM fleet_doc_snapshots WHERE model = ?",
        )
        .bind(model)
    };
    let rows = q
        .fetch_all(fleet.pool().inner())
        .await
        .map_err(crate::error::FleetError::Db)?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let score: f64 = row.get("s");
            SnapRow {
                node_id: row.get("node_id"),
                snapshot_score: score as f32,
                snapshot_at: row.get("at"),
            }
        })
        .collect())
}

pub(super) fn average_cosine(
    src: &[f32],
    targets: &[(String, String)],
    embed_by_key: &HashMap<(String, String), Vec<f32>>,
) -> Option<(f32, u32)> {
    let na = vec_norm(src);
    if na == 0.0 {
        return None;
    }
    let mut total = 0.0f32;
    let mut count = 0u32;
    for key in targets {
        let Some(v) = embed_by_key.get(key) else {
            continue;
        };
        if v.len() != src.len() {
            continue;
        }
        let nb = vec_norm(v);
        if nb == 0.0 {
            continue;
        }
        let dot: f32 = src.iter().zip(v).map(|(x, y)| x * y).sum();
        total += (dot / (na * nb)).clamp(-1.0, 1.0);
        count += 1;
    }
    if count == 0 {
        None
    } else {
        Some((total / count as f32, count))
    }
}

fn vec_norm(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}
