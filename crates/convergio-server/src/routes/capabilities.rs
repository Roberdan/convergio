//! `/v1/capabilities/*` local capability registry routes.

use crate::app::AppState;
use crate::capability_install::{install_file, InstallFileRequest};
use crate::capability_remote::{install_remote, InstallRemoteRequest};
use crate::error::ApiError;
use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use convergio_durability::{
    Capability, CapabilitySignatureRequest, CapabilitySignatureVerification, DurabilityError,
};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum InstallRequest {
    Local(InstallFileRequest),
    Remote {
        #[serde(default)]
        remote: bool,
        registry_url: String,
        name: String,
        #[serde(default)]
        version: Option<String>,
    },
}

/// Mount capability registry routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/v1/capabilities", get(list))
        .route("/v1/capabilities/install-file", post(install))
        .route("/v1/capabilities/verify-signature", post(verify_signature))
        .route("/v1/capabilities/:name/disable", post(disable))
        .route("/v1/capabilities/:name", get(get_one).delete(remove))
}

async fn list(State(state): State<AppState>) -> Result<Json<Vec<Capability>>, ApiError> {
    Ok(Json(state.durability.capabilities().list().await?))
}

async fn get_one(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Capability>, ApiError> {
    Ok(Json(state.durability.capabilities().get(&name).await?))
}

async fn verify_signature(
    State(state): State<AppState>,
    Json(body): Json<CapabilitySignatureRequest>,
) -> Result<Json<CapabilitySignatureVerification>, ApiError> {
    Ok(Json(
        state.durability.verify_capability_signature(body).await?,
    ))
}

async fn install(
    State(state): State<AppState>,
    Json(body): Json<InstallRequest>,
) -> Result<Json<Capability>, ApiError> {
    let installed = match body {
        InstallRequest::Local(body) => install_file(&state.durability, body).await?,
        InstallRequest::Remote {
            remote,
            registry_url,
            name,
            version,
        } => {
            if !remote {
                return Err(DurabilityError::InvalidCapability {
                    reason: "remote install request must set remote=true".into(),
                }
                .into());
            }
            install_remote(
                &state.durability,
                InstallRemoteRequest {
                    registry_url,
                    name,
                    version,
                },
            )
            .await?
        }
    };
    Ok(Json(installed))
}

async fn disable(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Capability>, ApiError> {
    Ok(Json(
        state
            .durability
            .set_capability_status(&name, "disabled")
            .await?,
    ))
}

async fn remove(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let cap = state.durability.capabilities().get(&name).await?;
    if cap.status == "enabled" {
        return Err(DurabilityError::InvalidCapability {
            reason: "disable capability before removal".into(),
        }
        .into());
    }
    if let Some(root) = &cap.root_path {
        let root = std::path::Path::new(root);
        if root.exists() {
            std::fs::remove_dir_all(root).map_err(|e| DurabilityError::InvalidCapability {
                reason: format!("failed to remove capability files: {e}"),
            })?;
        }
    }
    let removed = state.durability.remove_capability(&name).await?;
    Ok(Json(
        json!({"removed": removed.name, "version": removed.version}),
    ))
}
