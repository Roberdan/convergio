//! Branch merge implementation (overlay -> mainline).

use crate::audit::{append_tx, EntityKind};
use crate::error::{DurabilityError, Result};
use crate::{OntologyBranch, OntologyBranchStatus};
use chrono::Utc;
use serde_json::json;

impl crate::Durability {
    pub(crate) async fn merge_ontology_branch(
        &self,
        branch_id: &str,
        current: OntologyBranch,
        agent_id: Option<&str>,
    ) -> Result<OntologyBranch> {
        if current.status != OntologyBranchStatus::Review {
            return Err(DurabilityError::IllegalOntologyBranchTransition {
                from: current.status.as_str(),
                to: OntologyBranchStatus::Merged.as_str(),
            });
        }

        let now = Utc::now();
        let mut tx = self.pool().inner().begin().await?;

        let overlay = sqlx::query_as::<_, (String, String, Option<String>)>(
            "SELECT key, op_kind, value FROM ontology_branch_entries WHERE branch_id = ? ORDER BY key ASC",
        )
        .bind(branch_id)
        .fetch_all(&mut *tx)
        .await?;

        let mut set_count = 0usize;
        let mut delete_count = 0usize;

        for (key, op_kind, value) in overlay {
            match op_kind.as_str() {
                "set" => {
                    let raw = value.ok_or_else(|| DurabilityError::InvalidOntologyEntry {
                        reason: "overlay op_kind=set requires value".into(),
                    })?;
                    sqlx::query(
                        "INSERT INTO ontology_entries (key, value, updated_at) VALUES (?, ?, ?) \
                         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                    )
                    .bind(&key)
                    .bind(raw)
                    .bind(now.to_rfc3339())
                    .execute(&mut *tx)
                    .await?;
                    set_count += 1;
                }
                "delete" => {
                    sqlx::query("DELETE FROM ontology_entries WHERE key = ?")
                        .bind(&key)
                        .execute(&mut *tx)
                        .await?;
                    delete_count += 1;
                }
                other => {
                    return Err(DurabilityError::InvalidOntologyEntry {
                        reason: format!("invalid overlay op_kind: {other}"),
                    });
                }
            }
        }

        let updated = sqlx::query(
            "UPDATE ontology_branches SET status = 'merged', updated_at = ?, merged_at = COALESCE(merged_at, ?) \
             WHERE id = ?",
        )
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(branch_id)
        .execute(&mut *tx)
        .await?;
        if updated.rows_affected() == 0 {
            return Err(DurabilityError::NotFound {
                entity: "ontology_branch",
                id: branch_id.to_string(),
            });
        }

        append_tx(
            &mut tx,
            EntityKind::Ontology,
            branch_id,
            "ontology.branch_merged",
            &json!({
                "branch_id": branch_id,
                "from": current.status,
                "to": "merged",
                "overlay": {"set": set_count, "delete": delete_count},
            }),
            agent_id,
        )
        .await?;

        tx.commit().await?;

        self.ontology().get_branch(branch_id).await
    }
}
