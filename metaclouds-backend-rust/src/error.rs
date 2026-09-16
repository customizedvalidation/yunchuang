//! Unified error type for the HTTP API.
//!
//! Every error maps to the shared envelope:
//! `{"success":false,"message":"...","code":"...","timestamp":<unix_sec>}`
//!
//! The `code` string and its HTTP status are aligned 1:1 with the Go v1 contract.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::response::ApiResponse;

/// Stable error codes used by the envelope `code` field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    InternalServerError,
    ValidationError,
}

impl ErrorCode {
    /// Wire-level string. Must stay byte-identical to the Go contract.
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::BadRequest => "BAD_REQUEST",
            ErrorCode::Unauthorized => "UNAUTHORIZED",
            ErrorCode::Forbidden => "FORBIDDEN",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::InternalServerError => "INTERNAL_SERVER_ERROR",
            ErrorCode::ValidationError => "VALIDATION_ERROR",
        }
    }

    pub fn http_status(self) -> StatusCode {
        match self {
            ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
            ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Conflict => StatusCode::CONFLICT,
            ErrorCode::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::ValidationError => StatusCode::BAD_REQUEST,
        }
    }
}

/// Application-level error carrying a stable code, a human-readable message
/// and (optionally) the underlying source error for logging.
#[derive(Debug)]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source
            .as_ref()
            .map(|e| e.as_ref() as &(dyn std::error::Error + 'static))
    }
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(
        code: ErrorCode,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::BadRequest, msg)
    }
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unauthorized, msg)
    }
    pub fn forbidden(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Forbidden, msg)
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, msg)
    }
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Conflict, msg)
    }
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::ValidationError, msg)
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalServerError, msg)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Never leak internal details to the client; log them server-side.
        if self.code == ErrorCode::InternalServerError {
            tracing::error!(error = %self, source = ?self.source, "internal server error");
        } else {
            tracing::warn!(code = self.code.as_str(), message = %self.message, "request failed");
        }
        let body = ApiResponse::<()> {
            success: false,
            data: None,
            message: Some(self.message.clone()),
            code: Some(self.code.as_str().to_string()),
            timestamp: chrono::Utc::now().timestamp(),
        };
        (self.code.http_status(), axum::Json(body)).into_response()
    }
}

// Allow `?` on sqlx / other errors in handlers: they become 500 unless mapped.
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::not_found("resource not found"),
            other => AppError::with_source(ErrorCode::InternalServerError, "database error", other),
        }
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(e: argon2::password_hash::Error) -> Self {
        // `password_hash::Error` does not implement `std::error::Error` on
        // all feature combinations, so we preserve it as a string source.
        AppError::new(
            ErrorCode::InternalServerError,
            format!("password hashing error: {e}"),
        )
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        AppError::with_source(ErrorCode::Unauthorized, "invalid or expired token", e)
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(e: validator::ValidationErrors) -> Self {
        AppError::new(
            ErrorCode::ValidationError,
            format!("validation failed: {e}"),
        )
    }
}

/// Helper so handlers can return `Result<Json<...>, AppError>`.
pub type AppResult<T> = Result<T, AppError>;
