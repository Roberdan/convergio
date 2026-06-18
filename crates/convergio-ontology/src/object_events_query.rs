//! Bitemporal as-of read queries for the `object_events` log (ADR-0053, W3).
//!
//! The companion [`crate::object_events`] module only ever returns the
//! *current* state (`object_events_tx_current`). This module finally
//! queries the two temporal axes the table was built for:
//!
//! - **valid-time** (`valid_from` / `valid_to`) — when a fact is true in
//!   the modelled world. [`ObjectEventsStore::get_valid_as_of`] /
//!   [`ObjectEventsStore::list_valid_as_of`] answer "what did we believe
//!   was true *at* `valid_at`?" among the transaction-current rows
//!   (`tx_to IS NULL`).
//! - **transaction-time** (`tx_from` / `tx_to`) — when the system knew a
//!   fact. [`ObjectEventsStore::get_tx_as_of`] /
//!   [`ObjectEventsStore::list_tx_as_of`] answer "what did the system
//!   *know* at `tx_at`?".
//!
//! Timestamps are bound as RFC3339 strings to match the
//! [`crate::object_events::ObjectEventsStore::append_event`] write path;
//! lexicographic comparison is correct because every persisted value is a
//! UTC `to_rfc3339()` rendering with an identical offset/precision shape.

use crate::error::Result;
use crate::object_events::{row_to_event, ObjectEvent, ObjectEventsStore};
use chrono::{DateTime, Utc};

const SELECT_COLS: &str =
    "SELECT object_id, op, payload, valid_from, valid_to, tx_from, tx_to FROM object_events";

impl ObjectEventsStore {
    /// Return the event whose **valid-time** window contains `valid_at`,
    /// among the transaction-current rows (`tx_to IS NULL`).
    ///
    /// Predicate: `valid_from <= valid_at AND (valid_to IS NULL OR valid_at
    /// < valid_to)`. Returns `None` when no transaction-current row was
    /// valid at that instant.
    pub async fn get_valid_as_of(
        &self,
        object_id: &str,
        valid_at: DateTime<Utc>,
    ) -> Result<Option<ObjectEvent>> {
        let at = valid_at.to_rfc3339();
        let row = sqlx::query(&format!(
            "{SELECT_COLS} WHERE object_id = ? AND tx_to IS NULL \
             AND valid_from <= ? AND (valid_to IS NULL OR ? < valid_to) \
             ORDER BY valid_from DESC LIMIT 1"
        ))
        .bind(object_id)
        .bind(&at)
        .bind(&at)
        .fetch_optional(self.pool.inner())
        .await?;
        row.map(row_to_event).transpose()
    }

    /// Return the event as it was **known at transaction-time** `tx_at`,
    /// picking the valid-current slice when several were known at once.
    ///
    /// Predicate: `tx_from <= tx_at AND (tx_to IS NULL OR tx_at < tx_to)`.
    /// Rows with an open valid-time window (`valid_to IS NULL`) are
    /// preferred so the result reflects the then-current belief.
    pub async fn get_tx_as_of(
        &self,
        object_id: &str,
        tx_at: DateTime<Utc>,
    ) -> Result<Option<ObjectEvent>> {
        let at = tx_at.to_rfc3339();
        let row = sqlx::query(&format!(
            "{SELECT_COLS} WHERE object_id = ? \
             AND tx_from <= ? AND (tx_to IS NULL OR ? < tx_to) \
             ORDER BY (valid_to IS NULL) DESC, valid_from DESC, tx_from DESC LIMIT 1"
        ))
        .bind(object_id)
        .bind(&at)
        .bind(&at)
        .fetch_optional(self.pool.inner())
        .await?;
        row.map(row_to_event).transpose()
    }

    /// Snapshot every object by **valid-time** at `valid_at`: the
    /// transaction-current row whose valid window contains `valid_at`.
    /// Ordered by `object_id` for deterministic output.
    pub async fn list_valid_as_of(&self, valid_at: DateTime<Utc>) -> Result<Vec<ObjectEvent>> {
        let at = valid_at.to_rfc3339();
        let rows = sqlx::query(&format!(
            "{SELECT_COLS} WHERE tx_to IS NULL \
             AND valid_from <= ? AND (valid_to IS NULL OR ? < valid_to) \
             ORDER BY object_id ASC"
        ))
        .bind(&at)
        .bind(&at)
        .fetch_all(self.pool.inner())
        .await?;
        rows.into_iter().map(row_to_event).collect()
    }

    /// Snapshot every object by **transaction-time** at `tx_at`: each row
    /// the system knew at that instant. Ordered by `object_id`, then
    /// `tx_from`, for deterministic output.
    pub async fn list_tx_as_of(&self, tx_at: DateTime<Utc>) -> Result<Vec<ObjectEvent>> {
        let at = tx_at.to_rfc3339();
        let rows = sqlx::query(&format!(
            "{SELECT_COLS} WHERE tx_from <= ? AND (tx_to IS NULL OR ? < tx_to) \
             ORDER BY object_id ASC, tx_from ASC"
        ))
        .bind(&at)
        .bind(&at)
        .fetch_all(self.pool.inner())
        .await?;
        rows.into_iter().map(row_to_event).collect()
    }
}
