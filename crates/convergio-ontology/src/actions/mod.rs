//! Typed action framework primitives.
//!
//! This module provides:
//! - `Action<Input, Output>` and `AsyncAction<Input, Output>` base traits.
//! - idempotency key helpers.
//! - compensation hook modeling.
//! - a small in-memory registry for dispatch + introspection.

mod base;
mod idempotency;
mod registry;

pub use base::{
    Action, ActionCompensationFuture, ActionCompensationResult, ActionContext, ActionDescriptor,
    ActionError, ActionMetadata, AsyncAction, CompensationHint,
};
pub use idempotency::{IdempotencyKey, IdempotencyKeyError};
pub use registry::{ActionExecution, ActionRegistry, RegistryError};

#[cfg(test)]
mod tests;
