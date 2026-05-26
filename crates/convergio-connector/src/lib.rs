//! # convergio-connector
//!
//! Connector SDK core (ADR-0057): a small trait surface plus
//! YAML crosswalk parsing and a sandboxed process runner.
//!
//! This crate intentionally does **not** ship vertical connectors.
//! Vertical bundles implement the trait (in-process) or expose the
//! connector protocol behind the sandboxed runner.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod backoff;
mod canonical_json;
mod connector;
mod contract;
mod crosswalk;
mod error;
mod process_runner;
/// Line-delimited JSON protocol for sandboxed connectors.
pub mod protocol;
mod rate_limit;
mod types;

pub use backoff::{BackoffPolicy, BackoffState};
pub use connector::{Connector, DiscoverItem, DiscoverRequest, Health, PullPage, PullRequest};
pub use contract::assert_basic_connector_contract;
pub use crosswalk::{Crosswalk, CrosswalkField, CrosswalkParseReport};
pub use error::{ConnectorError, FailureKind};
pub use process_runner::{ProcessConnector, ProcessConnectorSpec};
pub use types::{ConnectorId, SchemaHash, Watermark};
