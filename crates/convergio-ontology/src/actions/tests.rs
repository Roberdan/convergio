use super::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::pin::Pin;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
struct EchoIn {
    value: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
struct EchoOut {
    value: String,
}

struct Echo;

impl AsyncAction for Echo {
    type Input = EchoIn;
    type Output = EchoOut;

    const NAME: &'static str = "test.echo";

    const METADATA: ActionMetadata = ActionMetadata {
        purpose_id: "test",
        required_roles: &["tester"],
        dual_control_threshold: Some(2),
        gdpr_article: Some("5(1)(b)"),
    };

    fn idempotency_key(input: &Self::Input) -> Option<IdempotencyKey> {
        IdempotencyKey::new(format!("echo:{}", input.value)).ok()
    }

    fn compensation_action() -> Option<&'static str> {
        Some("test.echo.undo")
    }

    fn execute(
        _ctx: ActionContext,
        input: Self::Input,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Self::Output, ActionError>> + Send>> {
        Box::pin(async move { Ok(EchoOut { value: input.value }) })
    }

    fn execute_with_compensation(
        _ctx: ActionContext,
        input: Self::Input,
    ) -> ActionCompensationFuture<Self::Output> {
        Box::pin(async move {
            let value = input.value;
            Ok((
                EchoOut {
                    value: value.clone(),
                },
                Some(json!({"value": value})),
            ))
        })
    }
}

struct Upper;

impl Action for Upper {
    type Input = EchoIn;
    type Output = EchoOut;

    const NAME: &'static str = "test.upper";

    const METADATA: ActionMetadata = ActionMetadata {
        purpose_id: "test",
        required_roles: &["tester"],
        dual_control_threshold: None,
        gdpr_article: None,
    };

    fn idempotency_key(input: &Self::Input) -> Option<IdempotencyKey> {
        IdempotencyKey::new(format!("upper:{}", input.value)).ok()
    }

    fn execute(_ctx: ActionContext, input: Self::Input) -> Result<Self::Output, ActionError> {
        Ok(EchoOut {
            value: input.value.to_ascii_uppercase(),
        })
    }
}

#[tokio::test]
async fn registry_dispatch_sets_idempotency_when_missing() {
    let mut reg = ActionRegistry::new();
    reg.register_async::<Echo>().unwrap();

    let mut ctx = ActionContext::empty();
    ctx.purpose_id = Some("test".to_string());
    ctx.roles = vec!["tester".to_string()];
    ctx.dual_control_approvals = vec!["approver-a".to_string(), "approver-b".to_string()];

    let out = reg
        .execute("test.echo", ctx, json!({"value":"hi"}))
        .await
        .unwrap();

    assert_eq!(out.output["value"], "hi");
    assert_eq!(out.idempotency_key.unwrap().as_str(), "echo:hi");
    assert_eq!(out.compensation_action, Some("test.echo.undo"));
    assert_eq!(out.compensation.as_ref().unwrap().action, "test.echo.undo");
    assert_eq!(out.compensation.as_ref().unwrap().input["value"], "hi");

    let desc = reg.describe("test.echo").unwrap();
    assert_eq!(desc.name, "test.echo");
    assert_eq!(desc.metadata.purpose_id, "test");
    assert_eq!(desc.metadata.dual_control_threshold, Some(2));
    assert_eq!(desc.compensation_action, Some("test.echo.undo"));
    assert_eq!(
        reg.compensation_action("test.echo").unwrap(),
        Some("test.echo.undo")
    );
}

#[tokio::test]
async fn registry_executes_sync_action() {
    let mut reg = ActionRegistry::new();
    reg.register_sync::<Upper>().unwrap();

    let mut ctx = ActionContext::empty();
    ctx.roles = vec!["tester".to_string()];

    let out = reg
        .execute("test.upper", ctx, json!({"value":"hi"}))
        .await
        .unwrap();

    assert_eq!(out.output["value"], "HI");
    assert_eq!(out.idempotency_key.unwrap().as_str(), "upper:hi");
    assert_eq!(out.compensation_action, None);
    assert!(out.compensation.is_none());
}

#[test]
fn idempotency_key_validation_rejects_non_ascii() {
    assert!(IdempotencyKey::new("\u{2603}").is_err());
}
