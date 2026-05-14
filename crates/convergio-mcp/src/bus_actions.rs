//! Plan-scoped bus actions for `convergio.act`.

use crate::action_params::{remove_key, required_str};
use crate::bridge::Bridge;
use crate::http::invalid;
use convergio_api::AgentResponse;
use serde_json::Value;

impl Bridge {
    pub(crate) async fn publish_message(&self, mut params: Value) -> AgentResponse {
        let plan_id = match required_str(&params, "plan_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        if let Err(response) = required_str(&params, "topic") {
            return response;
        }
        if params.get("payload").is_none() {
            return invalid("missing param: payload".into());
        }
        remove_key(&mut params, "plan_id");
        self.post(&format!("/v1/plans/{plan_id}/messages"), params)
            .await
    }

    pub(crate) async fn poll_messages(&self, params: Value) -> AgentResponse {
        let path = match bus_poll_path(&params) {
            Ok(path) => path,
            Err(response) => return response,
        };
        self.get(&path).await
    }

    pub(crate) async fn ack_message(&self, mut params: Value) -> AgentResponse {
        let message_id = match required_str(&params, "message_id") {
            Ok(value) => value,
            Err(response) => return response,
        };
        remove_key(&mut params, "message_id");
        self.post(&format!("/v1/messages/{message_id}/ack"), params)
            .await
    }
}

fn bus_poll_path(params: &Value) -> Result<String, AgentResponse> {
    let plan_id = required_str(params, "plan_id")?;
    let topic = required_str(params, "topic")?;
    let cursor = optional_i64(params, "cursor")?;
    let limit = optional_i64(params, "limit")?;
    let mut path = format!(
        "/v1/plans/{plan_id}/messages?topic={}",
        encode_query_component(&topic)
    );
    if let Some(cursor) = cursor {
        path.push_str(&format!("&cursor={cursor}"));
    }
    if let Some(limit) = limit {
        path.push_str(&format!("&limit={limit}"));
    }
    Ok(path)
}

fn optional_i64(params: &Value, key: &str) -> Result<Option<i64>, AgentResponse> {
    match params.get(key) {
        Some(value) => value
            .as_i64()
            .map(Some)
            .ok_or_else(|| invalid(format!("{key} must be an integer"))),
        None => Ok(None),
    }
}

fn encode_query_component(input: &str) -> String {
    input
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bus_poll_path_encodes_topic_and_validates_numbers() {
        let path =
            bus_poll_path(&json!({"plan_id": "p1", "topic": "task:one two", "cursor": 7})).unwrap();
        assert_eq!(
            path,
            "/v1/plans/p1/messages?topic=task%3Aone%20two&cursor=7"
        );
        let err =
            bus_poll_path(&json!({"plan_id": "p1", "topic": "t", "limit": "bad"})).unwrap_err();
        assert_eq!(err.message, "limit must be an integer");
    }
}
