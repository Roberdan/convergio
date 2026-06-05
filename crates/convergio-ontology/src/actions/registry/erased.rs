//! Erased dispatch adapters for the action registry.

use crate::actions::base::{
    Action, ActionContext, ActionDescriptor, ActionError, ActionMetadata, AsyncAction,
};
use crate::actions::idempotency::IdempotencyKey;
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;

use super::RegistryError;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone)]
pub(super) struct ErasedExecution {
    pub(super) output: Value,
    pub(super) compensation_input: Option<Value>,
}

pub(super) trait ErasedAction: Send + Sync {
    fn metadata(&self) -> ActionMetadata;
    fn descriptor(&self) -> ActionDescriptor;
    fn compensation_action(&self) -> Option<&'static str>;

    fn idempotency_key_from_value(
        &self,
        input: &Value,
    ) -> Result<Option<IdempotencyKey>, RegistryError>;

    fn execute_boxed<'a>(
        &'a self,
        ctx: ActionContext,
        input: Value,
    ) -> BoxFuture<'a, Result<ErasedExecution, ActionError>>;
}

pub(super) struct AsyncActionAdapter<A: AsyncAction>(std::marker::PhantomData<A>);

impl<A: AsyncAction> AsyncActionAdapter<A> {
    pub(super) fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<A> ErasedAction for AsyncActionAdapter<A>
where
    A: AsyncAction,
    A::Input: DeserializeOwned + JsonSchema,
    A::Output: Serialize + JsonSchema,
{
    fn metadata(&self) -> ActionMetadata {
        A::METADATA
    }

    fn descriptor(&self) -> ActionDescriptor {
        A::descriptor()
    }

    fn compensation_action(&self) -> Option<&'static str> {
        A::compensation_action()
    }

    fn idempotency_key_from_value(
        &self,
        input: &Value,
    ) -> Result<Option<IdempotencyKey>, RegistryError> {
        let parsed: A::Input = serde_json::from_value(input.clone())
            .map_err(|e| RegistryError::InvalidInput(e.to_string()))?;
        Ok(A::idempotency_key(&parsed))
    }

    fn execute_boxed<'a>(
        &'a self,
        ctx: ActionContext,
        input: Value,
    ) -> BoxFuture<'a, Result<ErasedExecution, ActionError>> {
        let parsed: A::Input = match serde_json::from_value(input) {
            Ok(v) => v,
            Err(e) => {
                return Box::pin(
                    async move { Err(ActionError::new("invalid_input", e.to_string())) },
                );
            }
        };

        Box::pin(async move {
            let (out, compensation_input) = A::execute_with_compensation(ctx, parsed).await?;
            let output = serde_json::to_value(out)
                .map_err(|e| ActionError::new("invalid_output", e.to_string()))?;
            Ok(ErasedExecution {
                output,
                compensation_input,
            })
        })
    }
}

pub(super) struct SyncActionAdapter<A: Action>(std::marker::PhantomData<A>);

impl<A: Action> SyncActionAdapter<A> {
    pub(super) fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<A> ErasedAction for SyncActionAdapter<A>
where
    A: Action,
    A::Input: DeserializeOwned + JsonSchema,
    A::Output: Serialize + JsonSchema,
{
    fn metadata(&self) -> ActionMetadata {
        A::METADATA
    }

    fn descriptor(&self) -> ActionDescriptor {
        A::descriptor()
    }

    fn compensation_action(&self) -> Option<&'static str> {
        A::compensation_action()
    }

    fn idempotency_key_from_value(
        &self,
        input: &Value,
    ) -> Result<Option<IdempotencyKey>, RegistryError> {
        let parsed: A::Input = serde_json::from_value(input.clone())
            .map_err(|e| RegistryError::InvalidInput(e.to_string()))?;
        Ok(A::idempotency_key(&parsed))
    }

    fn execute_boxed<'a>(
        &'a self,
        ctx: ActionContext,
        input: Value,
    ) -> BoxFuture<'a, Result<ErasedExecution, ActionError>> {
        let parsed: A::Input = match serde_json::from_value(input) {
            Ok(v) => v,
            Err(e) => {
                return Box::pin(
                    async move { Err(ActionError::new("invalid_input", e.to_string())) },
                );
            }
        };

        Box::pin(async move {
            let (out, compensation_input) = A::execute_with_compensation(ctx, parsed)?;
            let output = serde_json::to_value(out)
                .map_err(|e| ActionError::new("invalid_output", e.to_string()))?;
            Ok(ErasedExecution {
                output,
                compensation_input,
            })
        })
    }
}
