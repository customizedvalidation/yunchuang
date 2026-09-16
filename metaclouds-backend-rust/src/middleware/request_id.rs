//! Per-request tracing: reads (or mints) an `X-Request-ID` header and
//! attaches it to the tracing span so every log line for this request can
//! be correlated.

use axum::extract::Request;
use axum::http::header::HeaderName;
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

static REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Middleware: ensure the request carries a stable `X-Request-ID` and add it
/// to the tracing span + response headers.
pub async fn set_request_id(mut request: Request, next: Next) -> Response {
    let rid = request
        .headers()
        .get(&REQUEST_ID)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    tracing::debug!(request_id = %rid, "handling request");

    // Stash it for handlers that want to read it directly.
    request.extensions_mut().insert(RequestId(rid.clone()));

    let mut response = next.run(request).await;
    if let Ok(value) = rid.parse() {
        response.headers_mut().insert(REQUEST_ID.clone(), value);
    }
    response
}

/// Extracted via `RequestId` extension in handlers if needed.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);
