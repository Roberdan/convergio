//! Constrained `convergio.act` action dispatch.

use crate::bridge::Bridge;
use crate::help;
use crate::http::{invalid, mismatch, ok};
use convergio_api::{ActRequest, Action, AgentResponse, NextHint, SCHEMA_VERSION};
use serde_json::{json, Value};

impl Bridge {
    pub(crate) async fn dispatch(&self, request: ActRequest) -> AgentResponse {
        if request.schema_version != SCHEMA_VERSION {
            return mismatch(request.schema_version);
        }

        let action = request.action;
        let response = match action {
            Action::Status => self.status().await,
            Action::CreatePlan => self.post("/v1/plans", request.params).await,
            Action::CreateTask => self.create_task(request.params).await,
            Action::ListTasks => self.list_tasks(request.params).await,
            Action::NextTask => self.next_task(request.params).await,
            Action::ClaimTask => self.transition(request.params, "in_progress").await,
            Action::Heartbeat => self.heartbeat(request.params).await,
            Action::AddEvidence => self.add_evidence(request.params).await,
            Action::GetTaskContext => self.task_context(request.params).await,
            Action::PublishMessage => self.publish_message(request.params).await,
            Action::PollMessages => self.poll_messages(request.params).await,
            Action::AckMessage => self.ack_message(request.params).await,
            Action::SubmitTask => self.transition(request.params, "submitted").await,
            Action::ValidatePlan => self.validate_plan(request.params).await,
            Action::AuditVerify => self.audit_verify(request.params).await,
            Action::AuditAppend => self.post("/v1/audit/append", request.params).await,
            Action::ImportCrdtOps => self.post("/v1/crdt/import", request.params).await,
            Action::ListCrdtConflicts => self.get("/v1/crdt/conflicts").await,
            Action::RegisterAgent => self.post("/v1/agent-registry/agents", request.params).await,
            Action::ListAgents => self.get("/v1/agent-registry/agents").await,
            Action::HeartbeatAgent => self.heartbeat_agent(request.params).await,
            Action::RetireAgent => self.retire_agent(request.params).await,
            Action::SpawnRunner => self.post("/v1/agents/spawn-runner", request.params).await,
            Action::PlannerSolve => {
                self.post("/v1/capabilities/planner/solve", request.params)
                    .await
            }
            Action::ListCapabilities => self.get("/v1/capabilities").await,
            Action::GetCapability => self.get_capability(request.params).await,
            Action::ClaimWorkspaceLease => self.post("/v1/workspace/leases", request.params).await,
            Action::ListWorkspaceLeases => self.get("/v1/workspace/leases").await,
            Action::ReleaseWorkspaceLease => self.release_workspace_lease(request.params).await,
            Action::SubmitPatchProposal => self.post("/v1/workspace/patches", request.params).await,
            Action::EnqueuePatchProposal => self.enqueue_patch_proposal(request.params).await,
            Action::ProcessMergeQueue => self.post("/v1/workspace/merge/next", json!({})).await,
            Action::ListMergeQueue => self.get("/v1/workspace/merge-queue").await,
            Action::ListWorkspaceConflicts => self.get("/v1/workspace/conflicts").await,
            Action::ExplainLastRefusal => self.explain_last_refusal().await,
            Action::AgentPrompt => ok("agent prompt", help::agent_prompt(), None),
        };
        self.log_action(action, &response);
        response
    }

    async fn status(&self) -> AgentResponse {
        self.get("/v1/status").await
    }

    async fn create_task(&self, mut params: Value) -> AgentResponse {
        let plan_id = match required_str(&params, "plan_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        remove_key(&mut params, "plan_id");
        self.post(&format!("/v1/plans/{plan_id}/tasks"), params)
            .await
    }

    async fn list_tasks(&self, params: Value) -> AgentResponse {
        let plan_id = match required_str(&params, "plan_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        self.get(&format!("/v1/plans/{plan_id}/tasks")).await
    }

    async fn next_task(&self, params: Value) -> AgentResponse {
        let list = self.list_tasks(params).await;
        if !list.ok {
            return list;
        }
        let task = list
            .data
            .as_ref()
            .and_then(|v| v.as_array())
            .and_then(|tasks| tasks.iter().find(|t| t["status"] == "pending"))
            .cloned();
        ok("next task selected", json!({ "task": task }), None)
    }

    async fn transition(&self, params: Value, target: &str) -> AgentResponse {
        let task_id = match required_str(&params, "task_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        let body = json!({
            "target": target,
            "agent_id": params.get("agent_id").and_then(Value::as_str),
        });
        self.post(&format!("/v1/tasks/{task_id}/transition"), body)
            .await
    }

    async fn heartbeat(&self, params: Value) -> AgentResponse {
        let task_id = match required_str(&params, "task_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        self.post(&format!("/v1/tasks/{task_id}/heartbeat"), json!({}))
            .await
    }

    async fn add_evidence(&self, params: Value) -> AgentResponse {
        let task_id = match required_str(&params, "task_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        self.post(&format!("/v1/tasks/{task_id}/evidence"), params)
            .await
    }

    async fn task_context(&self, mut params: Value) -> AgentResponse {
        let task_id = match required_str(&params, "task_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        remove_key(&mut params, "task_id");
        self.post(&format!("/v1/tasks/{task_id}/context"), params)
            .await
    }

    async fn validate_plan(&self, params: Value) -> AgentResponse {
        let plan_id = match required_str(&params, "plan_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        self.post(&format!("/v1/plans/{plan_id}/validate"), json!({}))
            .await
    }

    async fn audit_verify(&self, params: Value) -> AgentResponse {
        let path = match audit_path(&params) {
            Ok(value) => value,
            Err(response) => return response,
        };
        self.get(&path).await
    }

    async fn release_workspace_lease(&self, params: Value) -> AgentResponse {
        let lease_id = match required_str(&params, "lease_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        self.post(
            &format!("/v1/workspace/leases/{lease_id}/release"),
            json!({}),
        )
        .await
    }

    async fn heartbeat_agent(&self, mut params: Value) -> AgentResponse {
        let agent_id = match required_str(&params, "agent_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        remove_key(&mut params, "agent_id");
        self.post(
            &format!("/v1/agent-registry/agents/{agent_id}/heartbeat"),
            params,
        )
        .await
    }

    async fn retire_agent(&self, params: Value) -> AgentResponse {
        let agent_id = match required_str(&params, "agent_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        self.post(
            &format!("/v1/agent-registry/agents/{agent_id}/retire"),
            json!({}),
        )
        .await
    }

    async fn enqueue_patch_proposal(&self, params: Value) -> AgentResponse {
        let proposal_id = match required_str(&params, "proposal_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        self.post(
            &format!("/v1/workspace/patches/{proposal_id}/enqueue"),
            json!({}),
        )
        .await
    }

    async fn get_capability(&self, params: Value) -> AgentResponse {
        let name = match required_str(&params, "name") {
            Ok(value) => value,
            Err(response) => return response,
        };
        self.get(&format!("/v1/capabilities/{name}")).await
    }

    async fn explain_last_refusal(&self) -> AgentResponse {
        let local = self.last_refusal.lock().await.clone();
        let task_id = local
            .as_ref()
            .and_then(|v| v.get("task_id"))
            .and_then(Value::as_str);
        let path = match task_id {
            Some(task_id) => format!("/v1/audit/refusals/latest?task_id={task_id}"),
            None => "/v1/audit/refusals/latest".into(),
        };
        let persisted = self.get(&path).await;
        if persisted.ok && persisted.data.as_ref().is_some_and(|v| !v.is_null()) {
            return ok(
                "last gate refusal",
                json!({"source": "daemon_audit", "refusal": persisted.data}),
                Some(NextHint::FixAddEvidenceRetrySubmit),
            );
        }
        match local {
            Some(refusal) => ok(
                "last gate refusal",
                json!({"source": "bridge_memory", "refusal": refusal}),
                Some(NextHint::FixAddEvidenceRetrySubmit),
            ),
            None => ok("no gate refusal recorded", json!({ "refusal": null }), None),
        }
    }
}

pub(crate) fn required_str(params: &Value, key: &str) -> Result<String, AgentResponse> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| invalid(format!("missing string param: {key}")))
}

fn audit_path(params: &Value) -> Result<String, AgentResponse> {
    let mut query = Vec::new();
    for key in ["from", "to"] {
        if let Some(value) = params.get(key) {
            let number = value
                .as_i64()
                .ok_or_else(|| invalid(format!("{key} must be an integer")))?;
            query.push(format!("{key}={number}"));
        }
    }
    if query.is_empty() {
        Ok("/v1/audit/verify".into())
    } else {
        Ok(format!("/v1/audit/verify?{}", query.join("&")))
    }
}

pub(crate) fn remove_key(value: &mut Value, key: &str) {
    if let Value::Object(map) = value {
        map.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use convergio_api::AgentCode;

    #[tokio::test]
    async fn act_rejects_schema_mismatch_before_network() {
        let bridge = Bridge::new("http://127.0.0.1:1".into());
        let response = bridge
            .dispatch(ActRequest {
                schema_version: "999".into(),
                action: Action::Status,
                params: json!({}),
            })
            .await;
        assert!(!response.ok);
        assert_eq!(response.code, AgentCode::SchemaVersionMismatch);
        assert_eq!(response.next, Some(NextHint::RefreshHelp));
    }

    #[test]
    fn audit_path_validates_numbers() {
        let path = audit_path(&json!({"from": 1, "to": 9})).unwrap();
        assert_eq!(path, "/v1/audit/verify?from=1&to=9");
        let err = audit_path(&json!({"from": "bad"})).unwrap_err();
        assert_eq!(err.code, AgentCode::InvalidRequest);
    }
}
