//! Aggregate counters for `/v1/status.telemetry`.
//!
//! Each counter is one indexed `COUNT(*)` — single SQL aggregate, no
//! joins. Cheap enough for the dashboard tick. The block is purely
//! additive on the wire so existing CLI callers ignore it.

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
}
