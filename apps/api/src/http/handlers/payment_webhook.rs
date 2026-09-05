use axum::{
    Extension, Json,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use market_bot_payment::{PaymentError, PaymentHandlingResult, WebhookHeaders};
use market_bot_shared::{ApiError, ErrorCode, RequestContext};
use serde::Serialize;
use uuid::Uuid;

use crate::app::AppState;

#[derive(Debug, Serialize)]
pub struct PaymentWebhookResponse {
    pub result: &'static str,
}

/// Accepts sandbox (and later production) payment webhooks.
///
/// Safety: the raw body is verified before any payment state changes. Provider
/// retries reuse the same event ID, so duplicate deliveries must return 200
/// without emitting another fulfillment outbox event.
pub async fn receive_payment_webhook(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<PaymentWebhookResponse>), (StatusCode, Json<ApiError>)> {
    let webhook_headers = headers_from_http(&headers);
    let result = state
        .payment_handler
        .handle_webhook(&state.payment_provider, &webhook_headers, &body)
        .await
        .map_err(|error| map_payment_error(error, context.request_id()))?;

    Ok((
        StatusCode::OK,
        Json(PaymentWebhookResponse {
            result: match result {
                PaymentHandlingResult::Applied => "applied",
                PaymentHandlingResult::Duplicate => "duplicate",
                PaymentHandlingResult::IgnoredStale => "ignored",
            },
        }),
    ))
}

fn headers_from_http(headers: &HeaderMap) -> WebhookHeaders {
    let mut webhook_headers = WebhookHeaders::new();
    for (name, value) in headers.iter() {
        if let Ok(value) = value.to_str() {
            webhook_headers.insert(name.as_str(), value);
        }
    }
    webhook_headers
}

fn map_payment_error(error: PaymentError, request_id: Uuid) -> (StatusCode, Json<ApiError>) {
    let (status, code) = match error {
        PaymentError::InvalidSignature => (StatusCode::UNAUTHORIZED, ErrorCode::Unauthorized),
        PaymentError::InvalidPayload
        | PaymentError::BlankEventId
        | PaymentError::EventOutsideTimeWindow => {
            (StatusCode::BAD_REQUEST, ErrorCode::InvalidInput)
        }
        PaymentError::PaymentNotFound | PaymentError::RefundNotFound => {
            (StatusCode::NOT_FOUND, ErrorCode::NotFound)
        }
        PaymentError::InvalidStatusTransition { .. }
        | PaymentError::OrderMismatch
        | PaymentError::OrderNotRefundable
        | PaymentError::PaymentNotRefundable => {
            (StatusCode::CONFLICT, ErrorCode::OrderStateInvalid)
        }
        PaymentError::ProviderTemporarilyUnavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::PaymentRequiresAction,
        ),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError),
    };

    (
        status,
        Json(ApiError::new(code, error.to_string(), request_id)),
    )
}
