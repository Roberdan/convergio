//! DAO for the `telemetry_series` table (migration 0013).
//!
//! Stores 1-minute resolution snapshots of [`crate::TelemetryCounters`]
//! with a 7-day rolling window.  Reads are served by
//! `GET /v1/telemetry/series`.

use crate::Result;
use chrono::{DateTime, Timelike, Utc};
use serde::Serialize;
use sqlx::SqlitePool;

/// One data-point in the time series.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct TelemetryPoint {
    /// RFC3339 minute-bucket timestamp (seconds always `:00Z`).
    pub bucket_ts: String,
    /// Metric name, e.g. `agents_active_24h`.
    pub metric: String,
    /// Counter value at the time of the snapshot.
    pub value: i64,
}

/// Store for the `telemetry_series` table.
///
/// All methods borrow the pool to remain cheap to construct — callers
/// hold no state beyond the shared pool clone.
pub struct TelemetrySeriesStore<'a> {
    pool: &'a SqlitePool,
}

impl<'a> TelemetrySeriesStore<'a> {
    /// Create a store bound to `pool`.
    pub fn new(pool: &'a SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert or update a single `(bucket_ts, metric)` pair.
    ///
    /// If a row with the same bucket and metric already exists its value
    /// is overwritten (idempotent for same-minute retries).
    pub async fn upsert(&self, bucket_ts: &str, metric: &str, value: i64) -> Result<()> {
        sqlx::query(
            "INSERT INTO telemetry_series (bucket_ts, metric, value) \
             VALUES (?, ?, ?) \
             ON CONFLICT(bucket_ts, metric) DO UPDATE SET value = excluded.value",
        )
        .bind(bucket_ts)
        .bind(metric)
        .bind(value)
        .execute(self.pool)
        .await?;
        Ok(())
    }

    /// Return points for `metric` at or after `since`, ordered by
    /// `bucket_ts` ascending.
    pub async fn query(&self, metric: &str, since: &DateTime<Utc>) -> Result<Vec<TelemetryPoint>> {
        let since_str = since.to_rfc3339();
        let rows = sqlx::query_as::<_, TelemetryPoint>(
            "SELECT bucket_ts, metric, value \
             FROM telemetry_series \
             WHERE metric = ? AND bucket_ts >= ? \
             ORDER BY bucket_ts ASC",
        )
        .bind(metric)
        .bind(&since_str)
        .fetch_all(self.pool)
        .await?;
        Ok(rows)
    }

    /// Delete all rows older than `cutoff`.  Returns the number of rows
    /// deleted.
    pub async fn prune(&self, cutoff: &DateTime<Utc>) -> Result<u64> {
        let cutoff_str = cutoff.to_rfc3339();
        let result = sqlx::query("DELETE FROM telemetry_series WHERE bucket_ts < ?")
            .bind(&cutoff_str)
            .execute(self.pool)
            .await?;
        Ok(result.rows_affected())
    }
}

/// Truncate `ts` to a whole-minute RFC3339 string: seconds are set to 0,
/// sub-seconds dropped, UTC timezone.
pub fn minute_bucket(ts: DateTime<Utc>) -> String {
    ts.with_second(0)
        .expect("0 is a valid second value")
        .with_nanosecond(0)
        .expect("0 is a valid nanosecond value")
        .to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn minute_bucket_truncates_seconds() {
        let ts = Utc.with_ymd_and_hms(2026, 5, 5, 10, 30, 47).unwrap();
        let bucket = minute_bucket(ts);
        assert_eq!(bucket, "2026-05-05T10:30:00+00:00");
    }

    #[test]
    fn minute_bucket_at_zero_seconds_unchanged() {
        let ts = Utc.with_ymd_and_hms(2026, 5, 5, 10, 30, 0).unwrap();
        let bucket = minute_bucket(ts);
        assert_eq!(bucket, "2026-05-05T10:30:00+00:00");
    }
}
