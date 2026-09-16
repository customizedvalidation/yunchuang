//! HTTP API 统一错误类型。
//!
//! 每个错误都映射到共享响应信封：
//! `{"success":false,"message":"...","code":"...","timestamp":<unix_sec>}`
//!
//! `code` 字符串与 HTTP 状态与 Go v1 契约逐字对齐
//! （见 Go `pkg/errors/errors.go`）。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::response::ApiResponse;

/// 信封 `code` 字段使用的稳定错误码。
///
/// 字符串与 HTTP 状态必须与 Go 版逐字一致。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    InternalServerError,
    ValidationError,
    RateLimit,
    ServiceUnavailable,
    /// 兜底未知错误。
    Unknown,
}

impl ErrorCode {
    /// 线上字符串，必须与 Go 契约逐字节一致。
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::BadRequest => "BAD_REQUEST",
            ErrorCode::Unauthorized => "UNAUTHORIZED",
            ErrorCode::Forbidden => "FORBIDDEN",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::InternalServerError => "INTERNAL_SERVER_ERROR",
            ErrorCode::ValidationError => "VALIDATION_ERROR",
            ErrorCode::RateLimit => "RATE_LIMIT_EXCEEDED",
            ErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            ErrorCode::Unknown => "UNKNOWN_ERROR",
        }
    }

    /// 映射到 HTTP 状态码（Validation 复用 400，与 Go 一致）。
    pub fn http_status(self) -> StatusCode {
        match self {
            ErrorCode::BadRequest => StatusCode::BAD_REQUEST,
            ErrorCode::Unauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::Forbidden => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::Conflict => StatusCode::CONFLICT,
            ErrorCode::InternalServerError => StatusCode::INTERNAL_SERVER_ERROR,
            ErrorCode::ValidationError => StatusCode::BAD_REQUEST,
            ErrorCode::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            ErrorCode::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

/// 应用级错误：携带稳定错误码、可读 message，以及（可选）底层来源错误用于日志。
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
    /// 通用构造函数（对齐 Go `errors.New(code, message)`）。
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

    /// 线上错误码字符串。
    pub fn code_str(&self) -> &'static str {
        self.code.as_str()
    }

    /// 该错误对应的 HTTP 状态码。
    pub fn http_status(&self) -> StatusCode {
        self.code.http_status()
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
    pub fn rate_limit(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::RateLimit, msg)
    }
    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::ServiceUnavailable, msg)
    }
    pub fn unknown(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::Unknown, msg)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 内部细节绝不外泄给客户端，仅在服务端记录日志。
        if self.code == ErrorCode::InternalServerError || self.code == ErrorCode::Unknown {
            tracing::error!(error = %self, source = ?self.source, "internal server error");
        } else {
            tracing::warn!(code = self.code.as_str(), message = %self.message, "request failed");
        }
        let body = ApiResponse::<()>::error(self.message.clone(), self.code.as_str());
        (self.code.http_status(), axum::Json(body)).into_response()
    }
}

// 在 handler 中用 `?` 传播 sqlx 错误时自动映射：
// - RowNotFound -> 404 NotFound
// - 唯一约束冲突   -> 409 Conflict
// - 其余          -> 500 InternalServer
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        match e {
            sqlx::Error::RowNotFound => AppError::not_found("resource not found"),
            sqlx::Error::Database(ref db) if db.is_unique_violation() => {
                AppError::conflict("resource already exists")
            }
            other => AppError::with_source(ErrorCode::InternalServerError, "database error", other),
        }
    }
}

impl From<argon2::password_hash::Error> for AppError {
    fn from(e: argon2::password_hash::Error) -> Self {
        // `password_hash::Error` 在部分 feature 组合下未实现 `std::error::Error`，
        // 因此以字符串形式保留来源。
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

/// 便捷别名：handler 可返回 `Result<Json<...>, AppError>`。
pub type AppResult<T> = Result<T, AppError>;
