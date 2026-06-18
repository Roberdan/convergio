//! # convergio-connector
//!
//! Connector SDK core (ADR-0057): a small trait surface plus
//! YAML crosswalk parsing and a sandboxed process runner.
//!
//! Alongside the SDK core it ships two production-quality **reference
//! source connectors** (see [`connectors`]): a CSV connector and an
//! HTTP-JSON connector. Both implement the [`Connector`] trait, preserve
//! source order for determinism, and are unit-testable offline (the
//! HTTP-JSON connector fetches through an injectable [`connectors::JsonFetcher`]).
//! Other vertical bundles still implement the trait in-process or expose the
//! connector protocol behind the sandboxed runner.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod backoff;
mod canonical_json;
mod connector;
pub mod connectors;
mod contract;
mod crosswalk;
mod error;
/// Federated query path + refusal gate over multiple connectors (ADR-0057).
pub mod federated;
mod process_runner;
/// Line-delimited JSON protocol for sandboxed connectors.
pub mod protocol;
mod rate_limit;
mod types;

pub use backoff::{BackoffPolicy, BackoffState};
pub use connector::{Connector, DiscoverItem, DiscoverRequest, Health, PullPage, PullRequest};
pub use connectors::{
    CsvConfig, CsvConnector, HttpJsonConfig, HttpJsonConnector, JsonFetcher, StaticJsonFetcher,
};
pub use contract::assert_basic_connector_contract;
pub use crosswalk::{Crosswalk, CrosswalkField, CrosswalkParseReport};
pub use error::{ConnectorError, FailureKind};
pub use federated::{
    FederatedExecutor, FederatedQuery, FederatedResult, FederatedSource, FederationPolicy,
    MergedRecord, Projection, SourceCount,
};
pub use process_runner::{ProcessConnector, ProcessConnectorSpec};
pub use types::{ConnectorId, SchemaHash, Watermark};
