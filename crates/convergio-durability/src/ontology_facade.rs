//! Ontology facade operations: scenario branches + CoW overlay.

use crate::audit::EntityKind;
use crate::error::{DurabilityError, Result};
use crate::{OntologyBranch, OntologyBranchStatus, OntologyResolvedEntry};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

impl crate::Durability {
    /// Create a new ontology branch in `draft`.
    pub async fn create_ontology_branch(
        &self,
        name: &str,
        agent_id: Option<&str>,
    ) -> Result<OntologyBranch> {
        let name = name.trim();
        if name.is_empty() {
            return Err(DurabilityError::OntologyBranchNameEmpty);
        }
        let now = Utc::now();
        let branch = OntologyBranch {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            status: OntologyBranchStatus::Draft,
            created_at: now,
            updated_at: now,
            reviewed_at: None,
            merged_at: None,
            discarded_at: None,
        };
        self.ontology().insert_branch(&branch).await?;
        self.audit()
            .append(
                EntityKind::Ontology,
                &branch.id,
                "ontology.branch_created",
                &json!({
                    "branch_id": branch.id,
                    "name": branch.name,
                    "status": branch.status,
                }),
                agent_id,
            )
            .await?;
        Ok(branch)
    }

    /// List branches.
    pub async fn list_ontology_branches(&self) -> Result<Vec<OntologyBranch>> {
        self.ontology().list_branches().await
    }

    /// Transition a branch through `draft -> review -> merged|discarded`.
    pub async fn transition_ontology_branch(
        &self,
        branch_id: &str,
        target: OntologyBranchStatus,
        agent_id: Option<&str>,
    ) -> Result<OntologyBranch> {
        let current = self.ontology().get_branch(branch_id).await?;
        validate_branch_transition(current.status, target)?;
        let now = Utc::now();

        match target {
            OntologyBranchStatus::Review => {
                let updated = self
                    .ontology()
                    .update_branch_status(branch_id, target, now, Some(now), None, None)
                    .await?;
                self.audit()
                    .append(
                        EntityKind::Ontology,
                        branch_id,
                        "ontology.branch_review",
                        &json!({
                            "branch_id": branch_id,
                            "from": current.status,
                            "to": target,
                        }),
                        agent_id,
                    )
                    .await?;
                Ok(updated)
            }
            OntologyBranchStatus::Discarded => {
                let updated = self
                    .ontology()
                    .update_branch_status(branch_id, target, now, None, None, Some(now))
                    .await?;
                self.audit()
                    .append(
                        EntityKind::Ontology,
                        branch_id,
                        "ontology.branch_discarded",
                        &json!({
                            "branch_id": branch_id,
                            "from": current.status,
                            "to": target,
                        }),
                        agent_id,
                    )
                    .await?;
                Ok(updated)
            }
            OntologyBranchStatus::Merged => {
                self.merge_ontology_branch(branch_id, current, agent_id)
                    .await
            }
            OntologyBranchStatus::Draft => {
                // Draft is only reachable at creation time.
                Err(DurabilityError::IllegalOntologyBranchTransition {
                    from: current.status.as_str(),
                    to: target.as_str(),
                })
            }
        }
    }

    /// Resolve an ontology entry in mainline or in a branch overlay.
    pub async fn resolve_ontology_entry(
        &self,
        key: &str,
        branch_id: Option<&str>,
    ) -> Result<OntologyResolvedEntry> {
        let key = key.trim();
        if key.is_empty() {
            return Err(DurabilityError::InvalidOntologyEntry {
                reason: "key must be non-empty".into(),
            });
        }
        if let Some(branch_id) = branch_id {
            // Validate branch exists for branch-scoped reads.
            self.ontology().get_branch(branch_id).await?;
        }
        self.ontology().resolve_entry(key, branch_id).await
    }

    /// Set an ontology entry.
    pub async fn set_ontology_entry(
        &self,
        key: &str,
        value: serde_json::Value,
        branch_id: Option<&str>,
        agent_id: Option<&str>,
    ) -> Result<()> {
        let key = key.trim();
        if key.is_empty() {
            return Err(DurabilityError::InvalidOntologyEntry {
                reason: "key must be non-empty".into(),
            });
        }
        let now = Utc::now();
        match branch_id {
            Some(branch_id) => {
                let branch = self.ontology().get_branch(branch_id).await?;
                ensure_branch_open(&branch)?;
                self.ontology()
                    .upsert_branch_entry(branch_id, key, &value, now)
                    .await?;
                self.audit()
                    .append(
                        EntityKind::Ontology,
                        branch_id,
                        "ontology.entry_set",
                        &json!({
                            "branch_id": branch_id,
                            "key": key,
                            "scope": "branch",
                        }),
                        agent_id,
                    )
                    .await?;
            }
            None => {
                self.ontology().upsert_main_entry(key, &value, now).await?;
                self.audit()
                    .append(
                        EntityKind::Ontology,
                        key,
                        "ontology.entry_set",
                        &json!({
                            "key": key,
                            "scope": "main",
                        }),
                        agent_id,
                    )
                    .await?;
            }
        }
        Ok(())
    }

    /// Delete an ontology entry (mainline delete or branch overlay delete).
    pub async fn delete_ontology_entry(
        &self,
        key: &str,
        branch_id: Option<&str>,
        agent_id: Option<&str>,
    ) -> Result<()> {
        let key = key.trim();
        if key.is_empty() {
            return Err(DurabilityError::InvalidOntologyEntry {
                reason: "key must be non-empty".into(),
            });
        }
        let now = Utc::now();
        match branch_id {
            Some(branch_id) => {
                let branch = self.ontology().get_branch(branch_id).await?;
                ensure_branch_open(&branch)?;
                self.ontology()
                    .delete_branch_entry(branch_id, key, now)
                    .await?;
                self.audit()
                    .append(
                        EntityKind::Ontology,
                        branch_id,
                        "ontology.entry_deleted",
                        &json!({
                            "branch_id": branch_id,
                            "key": key,
                            "scope": "branch",
                        }),
                        agent_id,
                    )
                    .await?;
            }
            None => {
                self.ontology().delete_main_entry(key).await?;
                self.audit()
                    .append(
                        EntityKind::Ontology,
                        key,
                        "ontology.entry_deleted",
                        &json!({
                            "key": key,
                            "scope": "main",
                        }),
                        agent_id,
                    )
                    .await?;
            }
        }
        Ok(())
    }
}

fn ensure_branch_open(branch: &OntologyBranch) -> Result<()> {
    if branch.status.is_open() {
        Ok(())
    } else {
        Err(DurabilityError::OntologyBranchClosed {
            id: branch.id.clone(),
            status: branch.status.as_str().to_string(),
        })
    }
}

fn validate_branch_transition(from: OntologyBranchStatus, to: OntologyBranchStatus) -> Result<()> {
    let ok = matches!(
        (from, to),
        (OntologyBranchStatus::Draft, OntologyBranchStatus::Review)
            | (OntologyBranchStatus::Draft, OntologyBranchStatus::Discarded)
            | (OntologyBranchStatus::Review, OntologyBranchStatus::Merged)
            | (
                OntologyBranchStatus::Review,
                OntologyBranchStatus::Discarded
            )
    );
    if ok {
        Ok(())
    } else {
        Err(DurabilityError::IllegalOntologyBranchTransition {
            from: from.as_str(),
            to: to.as_str(),
        })
    }
}
