//! In-memory action registry.

mod erased;

use crate::actions::base::{
    Action, ActionContext, ActionDescriptor, ActionMetadata, AsyncAction, CompensationHint,
};
use crate::actions::idempotency::IdempotencyKey;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

use self::erased::{AsyncActionAdapter, ErasedAction, SyncActionAdapter};

/// Successful action execution result.
#[derive(Debug, Clone)]
pub struct ActionExecution {
    /// JSON output value.
    pub output: Value,
    /// Effective idempotency key used for this execution.
    pub idempotency_key: Option<IdempotencyKey>,
    /// Declared compensation action name, if any.
    pub compensation_action: Option<&'static str>,
    /// Optional compensation hint (action name + input).
    pub compensation: Option<CompensationHint>,
}

/// Errors raised by [`ActionRegistry`].
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// Action name collision.
    #[error("duplicate_action:{0}")]
    DuplicateAction(&'static str),
    /// Requested action is not registered.
    #[error("unknown_action:{0}")]
    UnknownAction(String),
    /// Input JSON does not match the action's input type.
    #[error("invalid_input:{0}")]
    InvalidInput(String),
    /// Caller purpose does not match the action's required purpose.
    #[error("purpose_mismatch:expected:{expected}:got:{got}")]
    PurposeMismatch {
        /// Expected purpose id.
        expected: &'static str,
        /// Provided purpose id.
        got: String,
    },
    /// Caller is missing required roles.
    #[error("forbidden_missing_roles:{missing}")]
    ForbiddenMissingRoles {
        /// Comma-separated missing role list.
        missing: String,
    },
    /// Dual-control approvals are insufficient.
    #[error("dual_control_insufficient:required:{required}:got:{got}")]
    DualControlInsufficient {
        /// Required approvals.
        required: u8,
        /// Provided distinct approvals.
        got: usize,
    },
    /// Action execution failed.
    #[error("action_failed:{code}:{message}")]
    ActionFailed {
        /// Stable error code.
        code: &'static str,
        /// Human/debug message.
        message: String,
    },
}

fn ensure_admitted(metadata: ActionMetadata, ctx: &ActionContext) -> Result<(), RegistryError> {
    if let Some(purpose_id) = ctx.purpose_id.as_deref() {
        if purpose_id != metadata.purpose_id {
            return Err(RegistryError::PurposeMismatch {
                expected: metadata.purpose_id,
                got: purpose_id.to_string(),
            });
        }
    }

    let missing: Vec<&'static str> = metadata
        .required_roles
        .iter()
        .copied()
        .filter(|r| !ctx.roles.iter().any(|have| have == r))
        .collect();

    if !missing.is_empty() {
        return Err(RegistryError::ForbiddenMissingRoles {
            missing: missing.join(","),
        });
    }

    if let Some(required) = metadata.dual_control_threshold {
        let approvals: BTreeSet<&str> = ctx
            .dual_control_approvals
            .iter()
            .map(|s| s.as_str())
            .filter(|s| !s.is_empty())
            .collect();
        if approvals.len() < required as usize {
            return Err(RegistryError::DualControlInsufficient {
                required,
                got: approvals.len(),
            });
        }
    }

    Ok(())
}

/// Registry of available actions.
///
/// This is an in-memory structure intended to be built at process startup.
#[derive(Default)]
pub struct ActionRegistry {
    actions: BTreeMap<&'static str, Box<dyn ErasedAction>>,
}

impl ActionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register an async action.
    pub fn register_async<A: AsyncAction>(&mut self) -> Result<(), RegistryError>
    where
        A::Input: DeserializeOwned + JsonSchema,
        A::Output: Serialize + JsonSchema,
    {
        self.register_erased(A::NAME, Box::new(AsyncActionAdapter::<A>::new()))
    }

    /// Register a synchronous action.
    pub fn register_sync<A: Action>(&mut self) -> Result<(), RegistryError>
    where
        A::Input: DeserializeOwned + JsonSchema,
        A::Output: Serialize + JsonSchema,
    {
        self.register_erased(A::NAME, Box::new(SyncActionAdapter::<A>::new()))
    }

    fn register_erased(
        &mut self,
        name: &'static str,
        action: Box<dyn ErasedAction>,
    ) -> Result<(), RegistryError> {
        if self.actions.contains_key(name) {
            return Err(RegistryError::DuplicateAction(name));
        }
        self.actions.insert(name, action);
        Ok(())
    }

    /// Return descriptors for all registered actions.
    pub fn descriptors(&self) -> Vec<ActionDescriptor> {
        self.actions.values().map(|a| a.descriptor()).collect()
    }

    fn get(&self, name: &str) -> Option<&dyn ErasedAction> {
        self.actions.get(name).map(|b| b.as_ref())
    }

    /// Execute an action by name with JSON input.
    pub async fn execute(
        &self,
        name: &str,
        mut ctx: ActionContext,
        input: Value,
    ) -> Result<ActionExecution, RegistryError> {
        let action = self
            .get(name)
            .ok_or_else(|| RegistryError::UnknownAction(name.to_string()))?;

        ensure_admitted(action.metadata(), &ctx)?;

        if ctx.idempotency_key.is_none() {
            ctx.idempotency_key = action.idempotency_key_from_value(&input)?;
        }

        let idempotency_key = ctx.idempotency_key.clone();
        let compensation_action = action.compensation_action();

        let erased =
            action
                .execute_boxed(ctx, input)
                .await
                .map_err(|e| RegistryError::ActionFailed {
                    code: e.code,
                    message: e.message,
                })?;

        let compensation = match (compensation_action, erased.compensation_input) {
            (Some(action), Some(input)) => Some(CompensationHint { action, input }),
            _ => None,
        };

        Ok(ActionExecution {
            output: erased.output,
            idempotency_key,
            compensation_action,
            compensation,
        })
    }

    /// Compute the derived idempotency key for an action input without executing it.
    pub fn idempotency_key_for_input(
        &self,
        name: &str,
        input: &Value,
    ) -> Result<Option<IdempotencyKey>, RegistryError> {
        let action = self
            .get(name)
            .ok_or_else(|| RegistryError::UnknownAction(name.to_string()))?;
        action.idempotency_key_from_value(input)
    }

    /// Return registry introspection for one action.
    pub fn describe(&self, name: &str) -> Result<ActionDescriptor, RegistryError> {
        let action = self
            .get(name)
            .ok_or_else(|| RegistryError::UnknownAction(name.to_string()))?;
        Ok(action.descriptor())
    }

    /// Return a declared compensation action name, if any.
    pub fn compensation_action(&self, name: &str) -> Result<Option<&'static str>, RegistryError> {
        let action = self
            .get(name)
            .ok_or_else(|| RegistryError::UnknownAction(name.to_string()))?;
        Ok(action.compensation_action())
    }
}
