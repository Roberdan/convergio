//! Purpose-binding enforcement middleware.
//!
//! Enforces GDPR Art.5(1)(b): every request must be bound to a declared
//! processing purpose (`purpose_id`).

use crate::ApiError;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

/// Parsed purpose id attached to the request for downstream handlers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PurposeId(pub Uuid);

/// Reject any request without a valid `x-purpose-id` UUID header.
pub async fn enforce(mut req: Request, next: Next) -> Result<Response, ApiError> {
    let raw = match req.headers().get(convergio_api::PURPOSE_ID_HEADER) {
        Some(v) => v,
        None => {
            return Err(ApiError::BadRequest {
                code: "purpose_id_missing",
                message: format!(
                    "missing required header '{}'",
                    convergio_api::PURPOSE_ID_HEADER
                ),
            });
        }
    };

    let raw = raw.to_str().map_err(|_| ApiError::BadRequest {
        code: "purpose_id_invalid",
        message: format!(
            "header '{}' must be a valid UTF-8 string",
            convergio_api::PURPOSE_ID_HEADER
        ),
    })?;

    let raw = raw.trim();
    if raw.is_empty() {
        return Err(ApiError::BadRequest {
            code: "purpose_id_invalid",
            message: "purpose_id must not be empty".to_string(),
        });
    }

    let purpose_id = Uuid::parse_str(raw).map_err(|_| ApiError::BadRequest {
        code: "purpose_id_invalid",
        message: "purpose_id must be a UUID".to_string(),
    })?;

    req.extensions_mut().insert(PurposeId(purpose_id));
    Ok(next.run(req).await)
}
