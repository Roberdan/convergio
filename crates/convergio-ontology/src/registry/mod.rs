//! Versioned in-memory schema registry + migration policy.

mod error;
mod hash;
mod model;
mod store;

#[cfg(test)]
mod tests;

pub use self::error::RegistryError;
pub use self::model::{RegisteredSchema, SchemaSpec, SchemaSpecMeta};
pub use self::store::SchemaRegistry;
