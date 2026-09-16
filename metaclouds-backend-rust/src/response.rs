//! 统一 JSON 响应信封，所有 API 响应共用。
//!
//! 成功：
//! `{"success":true,"data":{...},"timestamp":<unix_sec>}`
//! 失败：
//! `{"success":false,"message":"...","code":"...","timestamp":<unix_sec>}`
//!
//! 与 Go 版 `pkg/response/response.go` 对齐的关键点：
//! - **成功响应的 `data` 字段始终存在**。即使没有数据（如删除成功），也会序列化为
//!   `"data":null`（通过 `Some(())` 实现），而不是整字段省略。
//! - **错误响应的 `data` 字段省略**（`Option::None` + skip）。

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    /// 成功时始终为 `Some`（`Some(())` 序列化为 `null`），保证 `data` 字段恒存在；
    /// 失败时为 `None`，由 `skip_serializing_if` 省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    pub timestamp: i64,
}

impl<T: Serialize> ApiResponse<T> {
    /// 成功信封（HTTP 200）。`data` 恒存在；无数据时传 `()` 得到 `"data":null`。
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
            code: None,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// 成功信封（HTTP 201 Created）。
    pub fn created(data: T) -> WithStatus<T> {
        WithStatus {
            status: StatusCode::CREATED,
            inner: Self::success(data),
        }
    }
}

impl ApiResponse<()> {
    /// 错误信封。HTTP 状态由调用方（通常是 `AppError::into_response`）决定。
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

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        axum::Json(self).into_response()
    }
}

/// 分页信封内层结构（对齐 Go `PaginatedResponse`）。
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub total_pages: i64,
}

impl<T: Serialize> PaginatedResponse<T> {
    /// 计算并构造分页结构（对齐 Go 版 `totalPages` 向上取整逻辑）。
    pub fn new(data: Vec<T>, total: i64, page: i64, page_size: i64) -> Self {
        let total_pages = if page_size <= 0 {
            0
        } else {
            (total + page_size - 1) / page_size
        };
        Self {
            data,
            total,
            page,
            page_size,
            total_pages,
        }
    }
}

/// 便捷包装：让 handler 可以返回带状态码的 `Json<ApiResponse<T>>`（如 201 Created）。
pub struct WithStatus<T: Serialize> {
    pub status: StatusCode,
    pub inner: ApiResponse<T>,
}

impl<T: Serialize> IntoResponse for WithStatus<T> {
    fn into_response(self) -> Response {
        (self.status, axum::Json(self.inner)).into_response()
    }
}
