//! LLM gateway cache store.
//!
//! Stores successful provider responses keyed by:
//! `(prompt_hash, model_id, retrieval_set_hash)`.

use chrono::{DateTime, Utc};
use convergio_db::Pool;
use serde_json::Value;
use sqlx::Row;

/// A single cached provider response.
#[derive(Debug, Clone)]
pub struct LlmGatewayCacheEntry {
    /// Provider identifier that produced the cached response.
    pub provider_id: String,
    /// Raw JSON response (provider-normalized by the gateway route).
    pub response: Value,
    /// Best-effort prompt token estimate (when available).
    pub input_tokens: Option<i64>,
    /// Best-effort completion token count (when available).
    pub output_tokens: Option<i64>,
    /// Cache insertion timestamp (RFC3339 / UTC).
    pub created_at: DateTime<Utc>,
}

/// DAO for the `llm_gateway_cache` table.
#[derive(Clone)]
pub struct LlmGatewayCacheStore {
    pool: Pool,
}

impl LlmGatewayCacheStore {
    /// New store handle.
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }

    /// Fetch cached response for the key.
    pub async fn get(
        &self,
        prompt_hash: &str,
        model_id: &str,
        retrieval_set_hash: &str,
    ) -> Result<Option<LlmGatewayCacheEntry>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT provider_id, response_json, input_tokens, output_tokens, created_at \
             FROM llm_gateway_cache \
             WHERE prompt_hash = ? AND model_id = ? AND retrieval_set_hash = ?",
        )
        .bind(prompt_hash)
        .bind(model_id)
        .bind(retrieval_set_hash)
        .fetch_optional(self.pool.inner())
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let created_at: String = row.try_get("created_at")?;
        let created_at = DateTime::parse_from_rfc3339(&created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let response_json: String = row.try_get("response_json")?;
        let response: Value = serde_json::from_str(&response_json).unwrap_or(Value::Null);

        Ok(Some(LlmGatewayCacheEntry {
            provider_id: row.try_get("provider_id")?,
            response,
            input_tokens: row.try_get("input_tokens")?,
            output_tokens: row.try_get("output_tokens")?,
            created_at,
        }))
    }

    /// Upsert a cached response.
    pub async fn put(
        &self,
        prompt_hash: &str,
        model_id: &str,
        retrieval_set_hash: &str,
        entry: &LlmGatewayCacheEntry,
    ) -> Result<(), sqlx::Error> {
        let created_at = entry.created_at.to_rfc3339();
        let response_json =
            serde_json::to_string(&entry.response).unwrap_or_else(|_| "null".into());

        sqlx::query(
            "INSERT INTO llm_gateway_cache \
             (prompt_hash, model_id, retrieval_set_hash, provider_id, response_json, input_tokens, output_tokens, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(prompt_hash, model_id, retrieval_set_hash) DO UPDATE SET \
               provider_id = excluded.provider_id, \
               response_json = excluded.response_json, \
               input_tokens = excluded.input_tokens, \
               output_tokens = excluded.output_tokens, \
               created_at = excluded.created_at",
        )
        .bind(prompt_hash)
        .bind(model_id)
        .bind(retrieval_set_hash)
        .bind(&entry.provider_id)
        .bind(&response_json)
        .bind(entry.input_tokens)
        .bind(entry.output_tokens)
        .bind(&created_at)
        .execute(self.pool.inner())
        .await?;

        Ok(())
    }
}
