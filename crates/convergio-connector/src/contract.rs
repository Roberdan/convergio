//! Contract-test helpers for connector implementations.

use crate::connector::{Connector, DiscoverRequest};
use crate::error::ConnectorError;

/// Minimal contract: connectors must be callable and stable on schema hash.
///
/// This is intentionally small; vertical connectors can add domain-specific
/// assertions.
pub async fn assert_basic_connector_contract<C: Connector>(c: &C) -> Result<(), ConnectorError> {
    let _ = c.health().await?;
    let h1 = c.schema_hash().await?;
    let h2 = c.schema_hash().await?;
    if h1 != h2 {
        return Err(ConnectorError::protocol("schema_hash must be stable"));
    }

    // `discover` should be callable even if it returns empty.
    let _ = c.discover(DiscoverRequest::default()).await?;
    Ok(())
}
