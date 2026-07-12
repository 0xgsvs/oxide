use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;
use validator::ValidationErrors;

/// Central application error type.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// Request body failed `validator` checks. Carries per-field errors.
    #[error("Validation failed")]
    Validation(#[source] ValidationErrors),

    /// A requested resource does not exist.
    #[error("Not found: {0}")]
    NotFound(String),

    /// Database query failed (details logged server-side).
    #[error("Database error")]
    Database(#[from] sqlx::Error),

    /// Authentication or authorization failure.
    #[error("{0}")]
    Unauthorized(&'static str),

    /// Resource conflict (e.g. duplicate email).
    #[error("{0}")]
    Conflict(&'static str),

    /// Generic bad request.
    #[error("{0}")]
    BadRequest(&'static str),

    /// Unexpected internal error.
    #[error("{0}")]
    Internal(&'static str),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            AppError::Validation(errors) => (
                StatusCode::BAD_REQUEST,
                json!({"error": "Validation failed", "fields": errors.field_errors()}),
            ),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, json!({"error": msg})),
            AppError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({"error": "Internal server error"}),
            ),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, json!({"error": msg})),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, json!({"error": msg})),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, json!({"error": msg})),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, json!({"error": msg})),
        };

        // Perf book: client errors are not server failures
        if status.is_client_error() {
            tracing::warn!(?self, status = ?status, "request failed");
        } else {
            tracing::error!(?self, status = ?status, "request failed");
        }
        (status, Json(body)).into_response()
    }
}
