//! MCP tool declarations for the bridge.

use crate::bridge_log;
use crate::help;
use crate::http::fallback_error;
use convergio_api::{ActRequest, Action, HelpRequest, HelpTopic, HelpVerbosity};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router, ServerHandler,
};
use schemars::JsonSchema;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub(crate) struct Bridge {
    pub(crate) url: String,
    pub(crate) client: reqwest::Client,
    pub(crate) last_refusal: Arc<Mutex<Option<Value>>>,
    pub(crate) tool_router: ToolRouter<Self>,
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
struct HelpParams {
    /// Help topic.
    #[serde(default = "default_help_topic")]
    topic: HelpTopic,
    /// Action details to return when topic is `action`.
    #[serde(default)]
    action: Option<Action>,
    /// Verbosity level.
    #[serde(default = "default_help_verbosity")]
    verbosity: HelpVerbosity,
}

impl From<HelpParams> for HelpRequest {
    fn from(value: HelpParams) -> Self {
        Self {
            topic: value.topic,
            action: value.action,
            verbosity: value.verbosity,
        }
    }
}

#[derive(Debug, serde::Deserialize, JsonSchema)]
struct ActParams {
    /// Schema version returned by `convergio.help`.
    schema_version: String,
    /// Action to execute.
    action: Action,
    /// Action-specific input.
    #[serde(default)]
    params: Value,
}

impl From<ActParams> for ActRequest {
    fn from(value: ActParams) -> Self {
        Self {
            schema_version: value.schema_version,
            action: value.action,
            params: value.params,
        }
    }
}

impl Bridge {
    pub(crate) fn new(url: String) -> Self {
        Self {
            url,
            client: reqwest::Client::new(),
            last_refusal: Arc::new(Mutex::new(None)),
            tool_router: Self::tool_router(),
        }
    }

    pub(crate) fn log_action(&self, action: Action, response: &convergio_api::AgentResponse) {
        bridge_log::append(action, response);
    }
}

#[tool_router(router = tool_router)]
impl Bridge {
    #[tool(
        name = "convergio.help",
        description = "Read Convergio agent protocol help."
    )]
    async fn help(&self, Parameters(params): Parameters<HelpParams>) -> String {
        if params.topic == HelpTopic::Actions {
            return convergio_api::actions_json().to_string();
        }
        serde_json::to_string(&help::response(&HelpRequest::from(params)))
            .unwrap_or_else(|e| fallback_error(format!("failed to serialize help response: {e}")))
    }

    #[tool(
        name = "convergio.act",
        description = "Execute one typed Convergio action."
    )]
    async fn act(&self, Parameters(params): Parameters<ActParams>) -> String {
        let response = self.dispatch(ActRequest::from(params)).await;
        serde_json::to_string(&response)
            .unwrap_or_else(|e| fallback_error(format!("failed to serialize action response: {e}")))
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for Bridge {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions("Use convergio.help once, then convergio.act with typed actions.")
    }
}

fn default_help_topic() -> HelpTopic {
    HelpTopic::Quickstart
}

fn default_help_verbosity() -> HelpVerbosity {
    HelpVerbosity::Short
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use convergio_bus::Bus;
    use convergio_db::Pool;
    use convergio_durability::{init, Durability};
    use convergio_lifecycle::Supervisor;
    use convergio_server::{router, AppState};
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tempfile::{tempdir, TempDir};
    use tokio::net::TcpListener;

    #[test]
    fn exposes_exact_two_tools() {
        let bridge = Bridge::new("http://127.0.0.1:8420".into());
        let tools = bridge.tool_router.list_all();
        let names: Vec<String> = tools.into_iter().map(|t| t.name.to_string()).collect();
        assert_eq!(names, vec!["convergio.act", "convergio.help"]);
    }

    #[tokio::test]
    async fn help_actions_matches_http_actions_byte_for_byte() {
        let (base_url, _dir) = boot_daemon().await;
        let bridge = Bridge::new(base_url.clone());

        let mcp = bridge
            .help(Parameters(HelpParams {
                topic: HelpTopic::Actions,
                action: None,
                verbosity: HelpVerbosity::Short,
            }))
            .await;

        let http = bridge
            .client
            .get(format!("{base_url}/v1/api/actions"))
            .send()
            .await
            .expect("GET /v1/api/actions")
            .bytes()
            .await
            .expect("actions bytes");

        assert_eq!(mcp.as_bytes(), http.as_ref());
    }

    async fn boot_daemon() -> (String, TempDir) {
        std::env::remove_var("CONVERGIO_EXECUTOR_USE_RUNNER");
        std::env::remove_var("CONVERGIO_EXECUTOR_MAX_PARALLEL");
        std::env::remove_var("CONVERGIO_REPO_PATH");

        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("state.db");
        let url = format!("sqlite://{}", db_path.display());
        let pool = Pool::connect(&url).await.expect("pool connect");
        init(&pool).await.expect("durability init");
        convergio_bus::init(&pool).await.expect("bus init");
        convergio_lifecycle::init(&pool)
            .await
            .expect("lifecycle init");
        let ontology = Arc::new(convergio_ontology::Store::new(pool.clone()));
        ontology.migrate().await.expect("ontology migrate");

        let state = AppState {
            durability: Arc::new(Durability::new(pool.clone())),
            bus: Arc::new(Bus::new(pool.clone())),
            supervisor: Arc::new(Supervisor::new(pool.clone())),
            graph: Arc::new(convergio_graph::Store::new(pool.clone())),
            embed: Arc::new(convergio_embed::EmbedStore::new(pool.clone())),
            embedder: Arc::new(
                convergio_embed::embedder::testing::DeterministicTestEmbedder::new(8),
            ),
            fleet: Arc::new(convergio_fleet::FleetStore::new(pool.clone())),
            fleet_plans: Arc::new(convergio_fleet::FleetPlanStore::new(pool.clone())),
            ontology,
            audit_verify_cache: Arc::new(std::sync::Mutex::new(None)),
        };
        let app: Router = router(state);

        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local_addr");
        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        (format!("http://{addr}"), dir)
    }
}
