//! Action base traits + shared metadata.

use crate::actions::idempotency::IdempotencyKey;
use schemars::{schema_for, JsonSchema};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

/// Per-invocation context passed to actions.
#[derive(Debug, Clone)]
pub struct ActionContext {
    /// Optional caller-supplied idempotency key.
    pub idempotency_key: Option<IdempotencyKey>,
    /// Optional active purpose binding (ADR-0054).
    pub purpose_id: Option<String>,
    /// Free-form actor identifier (service account, agent id, user id).
    pub actor: Option<String>,
    /// Effective roles attached to the actor for admission checks.
    pub roles: Vec<String>,
    /// Dual-control approver identities (distinct approvals).
    pub dual_control_approvals: Vec<String>,
}

impl ActionContext {
    /// Empty context.
    pub fn empty() -> Self {
        Self {
            idempotency_key: None,
            purpose_id: None,
            actor: None,
            roles: Vec::new(),
            dual_control_approvals: Vec::new(),
        }
    }
}

/// Compliance / policy metadata attached to an action type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ActionMetadata {
    /// Purpose registry ID that justifies invoking this action.
    pub purpose_id: &'static str,
    /// Required roles to invoke the action.
    pub required_roles: &'static [&'static str],
    /// Dual-control threshold (number of distinct approvals required).
    pub dual_control_threshold: Option<u8>,
    /// Related GDPR article reference (e.g. "5(1)(b)").
    pub gdpr_article: Option<&'static str>,
}

/// Introspection descriptor for an action implementation.
#[derive(Debug, Clone, Serialize)]
pub struct ActionDescriptor {
    /// Stable registry name.
    pub name: &'static str,
    /// Policy metadata.
    pub metadata: ActionMetadata,
    /// Declared compensation action name, if any.
    pub compensation_action: Option<&'static str>,
    /// JSON schema for the input.
    pub input_schema: Value,
    /// JSON schema for the output.
    pub output_schema: Value,
}

/// Optional hint describing how to compensate an action.
#[derive(Debug, Clone, Serialize)]
pub struct CompensationHint {
    /// Compensation action name.
    pub action: &'static str,
    /// Input payload for the compensation action.
    pub input: Value,
}

/// Structured action execution errors.
#[derive(Debug, thiserror::Error)]
#[error("{code}")]
pub struct ActionError {
    /// Stable machine-readable error code.
    pub code: &'static str,
    /// Human/debug message.
    pub message: String,
}

impl ActionError {
    /// Create a new action error.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// Standard output of an action that can be mechanically compensated.
pub type ActionCompensationResult<O> = Result<(O, Option<Value>), ActionError>;

/// Boxed future returned by [`AsyncAction::execute_with_compensation`].
pub type ActionCompensationFuture<O> =
    Pin<Box<dyn Future<Output = ActionCompensationResult<O>> + Send>>;

/// A synchronous typed action.
pub trait Action: Send + Sync + 'static {
    /// Typed input payload.
    type Input: DeserializeOwned + JsonSchema + Send + Sync + 'static;
    /// Typed output payload.
    type Output: Serialize + JsonSchema + Send + Sync + 'static;

    /// Stable action name used by the registry.
    const NAME: &'static str;

    /// Policy metadata (purpose/roles/dual-control/GDPR).
    const METADATA: ActionMetadata;

    /// Optional action-specific idempotency key derived from the input.
    ///
    /// If present, registry dispatch can use this as a stable replay key.
    fn idempotency_key(_input: &Self::Input) -> Option<IdempotencyKey> {
        None
    }

    /// Optional compensation action name that semantically reverses this action.
    ///
    /// Returning `None` means "irreversible" and should be explicitly
    /// justified by the caller wiring this action into admission gates.
    fn compensation_action() -> Option<&'static str> {
        None
    }

    /// Execute the action.
    fn execute(ctx: ActionContext, input: Self::Input) -> Result<Self::Output, ActionError>;

    /// Execute the action and optionally return a compensation input payload.
    ///
    /// Use this as the primary “compensation hook”: actions that declare a
    /// compensation action can override this method to produce the input
    /// required to undo their side effects.
    fn execute_with_compensation(
        ctx: ActionContext,
        input: Self::Input,
    ) -> Result<(Self::Output, Option<Value>), ActionError> {
        let out = Self::execute(ctx, input)?;
        Ok((out, None))
    }

    /// Build a self-contained descriptor (metadata + schemas).
    fn descriptor() -> ActionDescriptor {
        let input_schema =
            serde_json::to_value(schema_for!(Self::Input)).expect("input schema json");
        let output_schema =
            serde_json::to_value(schema_for!(Self::Output)).expect("output schema json");
        ActionDescriptor {
            name: Self::NAME,
            metadata: Self::METADATA,
            compensation_action: Self::compensation_action(),
            input_schema,
            output_schema,
        }
    }
}

/// Async variant of [`Action`].
///
/// Implementations return a boxed future to keep the trait object-safe.
pub trait AsyncAction: Send + Sync + 'static {
    /// Typed input payload.
    type Input: DeserializeOwned + JsonSchema + Send + Sync + 'static;
    /// Typed output payload.
    type Output: Serialize + JsonSchema + Send + Sync + 'static;

    /// Stable action name used by the registry.
    const NAME: &'static str;
    /// Policy metadata (purpose/roles/dual-control/GDPR).
    const METADATA: ActionMetadata;

    /// Optional idempotency key derived from the input.
    fn idempotency_key(_input: &Self::Input) -> Option<IdempotencyKey> {
        None
    }

    /// Optional compensation action name.
    fn compensation_action() -> Option<&'static str> {
        None
    }

    /// Execute the action asynchronously.
    fn execute(
        ctx: ActionContext,
        input: Self::Input,
    ) -> Pin<Box<dyn Future<Output = Result<Self::Output, ActionError>> + Send>>;

    /// Async variant of [`Action::execute_with_compensation`].
    fn execute_with_compensation(
        ctx: ActionContext,
        input: Self::Input,
    ) -> ActionCompensationFuture<Self::Output> {
        Box::pin(async move {
            let out = Self::execute(ctx, input).await?;
            Ok((out, None))
        })
    }

    /// Build a self-contained descriptor (metadata + schemas).
    fn descriptor() -> ActionDescriptor {
        let input_schema =
            serde_json::to_value(schema_for!(Self::Input)).expect("input schema json");
        let output_schema =
            serde_json::to_value(schema_for!(Self::Output)).expect("output schema json");
        ActionDescriptor {
            name: Self::NAME,
            metadata: Self::METADATA,
            compensation_action: Self::compensation_action(),
            input_schema,
            output_schema,
        }
    }
}
