//! Unified JSON envelope shared by every API response.
//!
//! Success:
//! `{"success":true,"data":{...},"timestamp":<unix_sec>}`
//! Error:
//! `{"success":false,"message":"...","code":"...","timestamp":<unix_sec>}`
//!
//! `data` / `message` / `code` are omitted when not applicable, matching the
//! Go v1 `omitempty` behavior.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub timestamp: i64,
}

impl<T: Serialize> ApiResponse<T> {
    /// Build a successful envelope carrying `data`.
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            code: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

impl ApiResponse<()> {
    /// Build an error envelope. The HTTP status is attached by the caller
    /// (usually `AppError::into_response`).
    pub fn error(message: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(message.into()),
            code: Some(code.into()),
            timestamp: chrono::Utc::now().timestamp(),
        }
    }
}

/// Convenience wrapper so handlers can return `Json<ApiResponse<T>>` with a
/// status code (e.g. 201 Created).
pub struct WithStatus<T: Serialize> {
    pub status: StatusCode,
    pub inner: ApiResponse<T>,
}

impl<T: Serialize> IntoResponse for WithStatus<T> {
    fn into_response(self) -> Response {
        (self.status, axum::Json(self.inner)).into_response()
    }
}
