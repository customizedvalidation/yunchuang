//! Panic boundary.
//!
//! axum already catches panics in handlers, but the default 500 body is not
//! our JSON envelope. This middleware wraps the remainder of the request
//! pipeline with `FutureExt::catch_unwind` so that a panic anywhere below us
//! is rewritten into the shared error envelope.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use futures_util::FutureExt;

use crate::error::ErrorCode;
use crate::response::ApiResponse;

pub async fn catch_panics(request: Request, next: Next) -> Response {
    let fut = std::panic::AssertUnwindSafe(next.run(request));

    match fut.catch_unwind().await {
        Ok(response) => response,
        Err(_) => {
            tracing::error!("panic in request handler");
            let body = ApiResponse::<()>::error(
                "internal server error",
                ErrorCode::InternalServerError.as_str(),
            );
            (
                ErrorCode::InternalServerError.http_status(),
                axum::Json(body),
            )
                .into_response()
        }
    }
}
