//! 错误信封与响应信封测试。
//!
//! 覆盖 6 类典型错误（400/401/403/404/409/500）的信封格式，
//! 并验证「成功响应 data 恒存在（含 null）、错误响应 data 省略」这一与 Go 版的对齐点。

use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

use metaclouds_backend_rust::error::{AppError, ErrorCode};
use metaclouds_backend_rust::response::ApiResponse;

/// 把 AppError 转成 (HTTP 状态, 解析后的 JSON)。
async fn envelope(err: AppError) -> (StatusCode, serde_json::Value) {
    let resp = err.into_response();
    let status = resp.status();
    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("read body");
    let v: serde_json::Value = serde_json::from_slice(&body).expect("decode json");
    (status, v)
}

#[tokio::test]
async fn error_envelope_fields() {
    let cases = [
        (
            AppError::bad_request("bad"),
            StatusCode::BAD_REQUEST,
            "BAD_REQUEST",
        ),
        (
            AppError::unauthorized("no"),
            StatusCode::UNAUTHORIZED,
            "UNAUTHORIZED",
        ),
        (
            AppError::forbidden("denied"),
            StatusCode::FORBIDDEN,
            "FORBIDDEN",
        ),
        (
            AppError::not_found("missing"),
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
        ),
        (AppError::conflict("dup"), StatusCode::CONFLICT, "CONFLICT"),
        (
            AppError::internal("boom"),
            StatusCode::INTERNAL_SERVER_ERROR,
            "INTERNAL_SERVER_ERROR",
        ),
    ];

    for (err, want_status, want_code) in cases {
        let (status, body) = envelope(err).await;
        assert_eq!(status, want_status);
        // success=false，message/code/timestamp 齐全。
        assert_eq!(body["success"], json!(false));
        assert!(body["message"].is_string());
        assert_eq!(body["code"], json!(want_code));
        assert!(body["timestamp"].is_i64());
        // 错误响应 data 字段必须省略。
        assert!(
            body.get("data").is_none(),
            "data should be omitted for {want_code}"
        );
    }
}

#[tokio::test]
async fn extended_error_codes_strings_and_status() {
    let cases = [
        (
            ErrorCode::RateLimit,
            StatusCode::TOO_MANY_REQUESTS,
            "RATE_LIMIT_EXCEEDED",
        ),
        (
            ErrorCode::ServiceUnavailable,
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
        ),
        (
            ErrorCode::Unknown,
            StatusCode::INTERNAL_SERVER_ERROR,
            "UNKNOWN_ERROR",
        ),
        (
            ErrorCode::ValidationError,
            StatusCode::BAD_REQUEST,
            "VALIDATION_ERROR",
        ),
    ];
    for (code, status, s) in cases {
        assert_eq!(code.as_str(), s);
        assert_eq!(code.http_status(), status);
        // 构造函数与 code_str / http_status 方法一致。
        let err = match code {
            ErrorCode::RateLimit => AppError::rate_limit("slow down"),
            ErrorCode::ServiceUnavailable => AppError::service_unavailable("down"),
            ErrorCode::Unknown => AppError::unknown("??"),
            ErrorCode::ValidationError => AppError::validation("bad field"),
            _ => unreachable!(),
        };
        assert_eq!(err.code_str(), s);
        assert_eq!(err.http_status(), status);
    }
}

#[tokio::test]
async fn success_envelope_always_has_data() {
    // 无数据成功：data 必须序列化为 null，而不是省略字段。
    let body = serde_json::to_value(ApiResponse::success(())).expect("serialize");
    assert_eq!(body["success"], json!(true));
    assert!(body["data"].is_null(), "data should be null, got {body}");
    assert!(body.get("message").is_none());
    assert!(body.get("code").is_none());
    assert!(body["timestamp"].is_i64());

    // 有数据成功：data 是对象。
    let body = serde_json::to_value(ApiResponse::success(json!({"id": 1}))).expect("serialize");
    assert_eq!(body["success"], json!(true));
    assert_eq!(body["data"]["id"], json!(1));
}

#[test]
fn created_helper_uses_201() {
    let resp = ApiResponse::created(json!({"id": 1})).into_response();
    assert_eq!(resp.status(), StatusCode::CREATED);
}
