//! Storage-backed deterministic resolver for [`super`].
//!
//! Splits the DB wiring (current-state projection over the
//! `object_properties` event log, grouping, and the reversible merge
//! record) out of `mod.rs` to keep both files under the 300-line cap.

use std::collections::BTreeMap;

use convergio_db::Pool;
use sqlx::Row;

use crate::error::Result;
use crate::object_storage::{LinkOp, ObjectLinkEvent, OntologyStore};

use super::{value_text, MatchGroup, MatchRule, MatchStrategy};

/// Link type used to record a reversible ER merge decision in the
/// append-only `object_links` log. `Add` asserts a merge, `Remove`
/// reverses it — so every merge stays auditable and undoable.
pub const SAME_AS_LINK_TYPE: &str = "er:same-as";

/// Accumulator for instances sharing one canonical key while grouping.
struct Bucket {
    fields: Vec<(String, String)>,
    members: Vec<String>,
}

/// Deterministic entity resolver backed by the shared SQLite pool.
///
/// Same constructor style as [`crate::OntologyStore`] /
/// [`crate::PurposeStore`]: bind it to an existing
/// [`convergio_db::Pool`] and call [`EntityResolver::candidates`].
#[derive(Clone)]
pub struct EntityResolver {
    pool: Pool,
}

impl EntityResolver {
    /// Bind the resolver to an existing SQLite pool.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Group instances of `object_type` under `tenant_id` that share the
    /// deterministic key declared by `rule`.
    ///
    /// Resolution runs over the **latest** value of each property (the
    /// `object_properties` event log is collapsed to current state, with
    /// the most recent `set` winning and a trailing `unset` removing the
    /// property). Only groups with two or more members are returned —
    /// singletons are not duplicate candidates.
    ///
    /// Output is deterministic: groups are ordered by their canonical key
    /// and members within each group are sorted ascending. Each
    /// [`MatchGroup`] carries a human-readable explanation of why its
    /// members matched.
    pub async fn candidates(
        &self,
        tenant_id: &str,
        object_type: &str,
        rule: &MatchRule,
    ) -> Result<Vec<MatchGroup>> {
        let states = self.current_property_states(tenant_id, object_type).await?;

        let mut buckets: BTreeMap<String, Bucket> = BTreeMap::new();
        for (object_id, props) in &states {
            if let Some(key) = rule.match_key(props) {
                let entry = buckets
                    .entry(key.canonical.clone())
                    .or_insert_with(|| Bucket {
                        fields: key.fields.clone(),
                        members: Vec::new(),
                    });
                entry.members.push(object_id.clone());
            }
        }

        let mut out = Vec::new();
        for (canonical, bucket) in buckets {
            let Bucket {
                fields,
                mut members,
            } = bucket;
            if members.len() < 2 {
                continue;
            }
            members.sort();
            let explanation = explain(object_type, &fields, members.len());
            out.push(MatchGroup {
                key: canonical,
                fields,
                members,
                explanation,
            });
        }
        Ok(out)
    }

    /// Project the `object_properties` event log to current state for
    /// every instance of `object_type` under `tenant_id`.
    ///
    /// Returns `object_id -> (property_name -> current textual value)`,
    /// keeping only properties whose latest event is a `set`.
    async fn current_property_states(
        &self,
        tenant_id: &str,
        object_type: &str,
    ) -> Result<BTreeMap<String, BTreeMap<String, String>>> {
        let rows = sqlx::query(
            "SELECT object_id, property_type, value_json FROM ( \
                 SELECT object_id, property_type, value_json, op, \
                        ROW_NUMBER() OVER ( \
                            PARTITION BY object_id, property_type \
                            ORDER BY created_at DESC, id DESC \
                        ) AS rn \
                 FROM object_properties \
                 WHERE tenant_id = ? \
                   AND object_id IN ( \
                       SELECT id FROM object_instances \
                       WHERE tenant_id = ? AND type = ? \
                   ) \
             ) WHERE rn = 1 AND op = 'set' \
             ORDER BY object_id, property_type",
        )
        .bind(tenant_id)
        .bind(tenant_id)
        .bind(object_type)
        .fetch_all(self.pool.inner())
        .await?;

        let mut states: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for row in rows {
            let object_id: String = row.try_get("object_id")?;
            let property_type: String = row.try_get("property_type")?;
            let value_json: String = row.try_get("value_json")?;
            states
                .entry(object_id)
                .or_default()
                .insert(property_type, value_text(&value_json));
        }
        Ok(states)
    }

    /// Record a reversible merge: assert an `er:same-as` link from the
    /// primary instance to the duplicate it absorbs.
    ///
    /// Reuses the append-only `object_links` log, so the decision is
    /// auditable and can be reversed with [`EntityResolver::record_unmerge`].
    /// Both ids must already exist under `tenant_id`.
    pub async fn record_merge(
        &self,
        tenant_id: &str,
        primary_id: &str,
        duplicate_id: &str,
    ) -> Result<ObjectLinkEvent> {
        OntologyStore::new(self.pool.clone())
            .append_link(
                tenant_id,
                primary_id,
                duplicate_id,
                SAME_AS_LINK_TYPE,
                LinkOp::Add,
            )
            .await
    }

    /// Reverse a previously recorded merge by appending a `remove` event
    /// for the `er:same-as` link. The log keeps both events, so the merge
    /// history stays fully auditable.
    pub async fn record_unmerge(
        &self,
        tenant_id: &str,
        primary_id: &str,
        duplicate_id: &str,
    ) -> Result<ObjectLinkEvent> {
        OntologyStore::new(self.pool.clone())
            .append_link(
                tenant_id,
                primary_id,
                duplicate_id,
                SAME_AS_LINK_TYPE,
                LinkOp::Remove,
            )
            .await
    }
}

/// Build the stable, human-readable explanation for a match group.
fn explain(object_type: &str, fields: &[(String, String)], members: usize) -> String {
    let props = fields
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let pairs = fields
        .iter()
        .map(|(k, v)| format!("{k}=\"{v}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{members} `{object_type}` instances share a deterministic key on [{props}]: {pairs}")
}
