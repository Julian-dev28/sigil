use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use serde_json::json;

use crate::store::StoreError;

/// A request-handling error that serializes to the API's error envelope.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// The request was malformed or failed validation.
    #[error("{0}")]
    InvalidRequest(String),

    /// The requested resource does not exist.
    #[error("{0}")]
    NotFound(String),

    /// A credential failed verification.
    #[error("{0}")]
    VerificationFailed(String),

    /// An unexpected internal error occurred.
    #[error("{0}")]
    Internal(String),
}

impl ApiError {
    fn code(&self) -> &'static str {
        match self {
            ApiError::InvalidRequest(_) => "invalid_request",
            ApiError::NotFound(_) => "not_found",
            ApiError::VerificationFailed(_) => "verification_failed",
            ApiError::Internal(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            ApiError::InvalidRequest(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::VerificationFailed(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Serialize)]
struct ErrorDetail {
    code: &'static str,
    message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(error = %self, "request failed");
        } else {
            tracing::debug!(error = %self, "request rejected");
        }
        let body = Json(json!(ErrorBody {
            error: ErrorDetail {
                code: self.code(),
                message: self.to_string(),
            }
        }));
        (status, body).into_response()
    }
}

impl From<StoreError> for ApiError {
    fn from(err: StoreError) -> Self {
        ApiError::Internal(format!("storage error: {err}"))
    }
}

impl From<sigil_core::CoreError> for ApiError {
    fn from(err: sigil_core::CoreError) -> Self {
        ApiError::Internal(format!("core error: {err}"))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(err: serde_json::Error) -> Self {
        ApiError::Internal(format!("serialization error: {err}"))
    }
}
