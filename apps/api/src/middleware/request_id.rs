use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode, header::HeaderName},
    middleware::Next,
    response::{IntoResponse, Response},
};
use market_bot_shared::RequestContext;
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "X-Request-Id";
pub const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// Attaches a normalized request context and returns its ID to the caller.
///
/// A caller-provided UUID is preserved; malformed or missing IDs are replaced
/// so every response and trace can still be correlated safely.
pub async fn request_context_middleware(mut request: Request, next: Next) -> Response {
    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap_or_else(Uuid::new_v4);

    let idempotency_key = match request.headers().get(IDEMPOTENCY_KEY_HEADER) {
        Some(value) => match value.to_str() {
            Ok(value) => Some(value.to_owned()),
            Err(_) => return invalid_idempotency_key_response(request_id),
        },
        None => None,
    };

    let context = match RequestContext::new(request_id, idempotency_key) {
        Ok(context) => context,
        Err(_) => return invalid_idempotency_key_response(request_id),
    };

    request.extensions_mut().insert(context);
    let mut response = next.run(request).await;
    let header_name = HeaderName::from_static("x-request-id");
    let header_value = HeaderValue::from_str(&request_id.to_string())
        .expect("UUID strings are always valid header values");
    response.headers_mut().insert(header_name, header_value);
    response
}

fn invalid_idempotency_key_response(request_id: Uuid) -> Response {
    let mut response = StatusCode::BAD_REQUEST.into_response();
    let header_value = HeaderValue::from_str(&request_id.to_string())
        .expect("UUID strings are always valid header values");
    response
        .headers_mut()
        .insert(HeaderName::from_static("x-request-id"), header_value);
    response
}
