//! Aggregate counters for `/v1/status.telemetry` and time-series
//! storage for `GET /v1/telemetry/series`.
//!
//! Each point-in-time counter is one indexed `COUNT(*)` — single SQL
//! aggregate, no joins. Cheap enough for the dashboard tick. The block
//! is purely additive on the wire so existing CLI callers ignore it.
//!
//! Time-series: the collector calls [`Durability::record_telemetry_snapshot`]
//! every 60 s.  That method upserts all 7 counters for the current
//! minute bucket and prunes rows older than 7 days.

use crate::store::telemetry_series::{minute_bucket, TelemetryPoint, TelemetrySeriesStore};
use crate::{Durability, Result};
use chrono::{Duration, Utc};
use serde::Serialize;

/// Coarse-grained activity counters surfaced on `/v1/status`. Used by
/// `cvg dash` and the multi-agent operating model to detect "no
/// agents are registering" without staring at audit rows.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TelemetryCounters {
    /// Every row in `agents` regardless of status.
    pub agents_registered_total: i64,
    /// Agents with `last_heartbeat_at` in the last 24 h.
    pub agents_active_24h: i64,
    /// `agent.session_started` audit rows in the last 24 h.
    pub sessions_started_24h: i64,
    /// Plans in `draft` or `active` status.
    pub plans_active: i64,
    /// Total audit rows (including pre-existing chain).
    pub audit_rows_total: i64,
    /// Bus messages created in the last 24 h.
    pub bus_messages_24h: i64,
    /// Workspace leases currently in `active` status.
    pub workspace_leases_active: i64,
}

impl Durability {
    /// Collect aggregate counters for `/v1/status`.
    pub async fn telemetry(&self) -> Result<TelemetryCounters> {
        let pool = self.pool().inner();
        let cutoff = (Utc::now() - Duration::hours(24)).to_rfc3339();
        let agents_registered_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM agents")
            .fetch_one(pool)
            .await
            .unwrap_or(0);
        let agents_active_24h: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM agents WHERE last_heartbeat_at >= ?")
                .bind(&cutoff)
                .fetch_one(pool)
                .await
                .unwrap_or(0);
        let sessions_started_24h: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM audit_log \
             WHERE transition = 'agent.session_started' AND created_at >= ?",
        )
        .bind(&cutoff)
        .fetch_one(pool)
        .await
        .unwrap_or(0);
        let plans_active: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM plans WHERE status IN ('draft','active')")
                .fetch_one(pool)
                .await
                .unwrap_or(0);
        let audit_rows_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM audit_log")
            .fetch_one(pool)
            .await
            .unwrap_or(0);
        let bus_messages_24h: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM agent_messages WHERE created_at >= ?")
                .bind(&cutoff)
                .fetch_one(pool)
                .await
                .unwrap_or(0);
        let workspace_leases_active: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM workspace_leases WHERE status = 'active'")
                .fetch_one(pool)
                .await
                .unwrap_or(0);
        Ok(TelemetryCounters {
            agents_registered_total,
            agents_active_24h,
            sessions_started_24h,
            plans_active,
            audit_rows_total,
            bus_messages_24h,
            workspace_leases_active,
        })
    }

    /// Snapshot the current counters into `telemetry_series` and prune
    /// rows older than 7 days.
    ///
    /// Called by the telemetry-collector loop every 60 s.  Safe to call
    /// from tests; the upsert is idempotent within the same minute.
    pub async fn record_telemetry_snapshot(&self) -> Result<()> {
        let now = Utc::now();
        let bucket = minute_bucket(now);
        let counters = self.telemetry().await?;

        let pool = self.pool().inner();
        let store = TelemetrySeriesStore::new(pool);

        let pairs: &[(&str, i64)] = &[
            ("agents_registered_total", counters.agents_registered_total),
            ("agents_active_24h", counters.agents_active_24h),
            ("sessions_started_24h", counters.sessions_started_24h),
            ("plans_active", counters.plans_active),
            ("audit_rows_total", counters.audit_rows_total),
            ("bus_messages_24h", counters.bus_messages_24h),
            ("workspace_leases_active", counters.workspace_leases_active),
        ];
        for (metric, value) in pairs {
            store.upsert(&bucket, metric, *value).await?;
        }

        let cutoff = now - Duration::days(7);
        store.prune(&cutoff).await?;
        Ok(())
    }

    /// Query the time series for a single metric over the last `window_days`
    /// days (capped at 7).
    ///
    /// Points are ordered by `bucket_ts` ascending.
    pub async fn query_telemetry_series(
        &self,
        metric: &str,
        window_days: u32,
    ) -> Result<Vec<TelemetryPoint>> {
        let days = window_days.min(7) as i64;
        let since = Utc::now() - Duration::days(days);
        let pool = self.pool().inner();
        let store = TelemetrySeriesStore::new(pool);
        store.query(metric, &since).await
    }
}
