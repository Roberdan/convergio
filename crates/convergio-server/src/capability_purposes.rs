//! Capability purpose-binding checks (ADR-0054 §B.2).
//!
//! A capability bundle (ADR-0008) may declare the processing purposes it is
//! bound to. The capability bucket says *what* an agent may do; a declared
//! purpose says *why*. A capability that declares no purpose is "ambient".
//! When it does declare purposes, each must reference a purpose registered in
//! the immutable purpose registry (`cvg purpose register`) so that a
//! vertical-level gate can reason about them — preventing a capability from
//! claiming an undeclared, unauditable purpose at install time.

use crate::ApiError;
use convergio_durability::Durability;

/// Refuse a capability whose declared purposes are not all registered.
///
/// An empty `declared` slice is allowed (ambient). Runs before any
/// filesystem mutation, so a rejected install leaves nothing behind.
pub(crate) async fn reject_unregistered(
    dur: &Durability,
    declared: &[String],
) -> Result<(), ApiError> {
    if declared.is_empty() {
        return Ok(());
    }
    let registered: std::collections::HashSet<String> =
        convergio_ontology::PurposeStore::new(dur.pool().clone())
            .list()
            .await?
            .into_iter()
            .map(|p| p.label)
            .collect();
    let unknown: Vec<String> = declared
        .iter()
        .filter(|p| !registered.contains(*p))
        .cloned()
        .collect();
    if unknown.is_empty() {
        Ok(())
    } else {
        Err(ApiError::BadRequest {
            code: "capability_purpose_unregistered",
            message: format!(
                "capability declares unregistered purpose(s): {}",
                unknown.join(", ")
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convergio_db::Pool;

    async fn pool() -> (Pool, tempfile::NamedTempFile) {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let url = format!("sqlite://{}", tmp.path().display());
        let pool = Pool::connect(&url).await.unwrap();
        convergio_durability::init(&pool).await.unwrap();
        convergio_ontology::Store::new(pool.clone())
            .migrate()
            .await
            .unwrap();
        (pool, tmp)
    }

    #[tokio::test]
    async fn empty_declaration_is_allowed() {
        let (pool, _tmp) = pool().await;
        let dur = Durability::new(pool);
        assert!(reject_unregistered(&dur, &[]).await.is_ok());
    }

    #[tokio::test]
    async fn registered_allowed_unregistered_refused() {
        let (pool, _tmp) = pool().await;
        convergio_ontology::PurposeStore::new(pool.clone())
            .register("student-records", "", None)
            .await
            .unwrap();
        let dur = Durability::new(pool);

        assert!(reject_unregistered(&dur, &["student-records".to_string()])
            .await
            .is_ok());

        let err = reject_unregistered(&dur, &["ghost-purpose".to_string()])
            .await
            .unwrap_err();
        match err {
            ApiError::BadRequest { code, .. } => {
                assert_eq!(code, "capability_purpose_unregistered")
            }
            _ => panic!("expected BadRequest variant"),
        }
    }
}
