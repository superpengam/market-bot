use axum::{
    Json,
    http::{HeaderMap, StatusCode},
};
use market_bot_cart::CartItemSource;
use market_bot_catalog::ProductStatus;
use market_bot_order::OrderStatus;
use market_bot_shared::{ApiError, ErrorCode, FulfillmentType, UserId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub mod ai;
pub mod cart;
pub mod moderation;
pub mod order;
pub mod payment_webhook;
pub mod product;
pub mod product_search;
pub mod seller;

/// Caller used when the `X-User-Id` header is omitted (local/dev and API tests).
pub const DEFAULT_TEST_USER_ID: Uuid = Uuid::from_u128(0x1111_1111_1111_4111_8111_1111_1111_1111);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiFulfillmentType {
    Digital,
    PhysicalStandard,
}

impl From<FulfillmentType> for ApiFulfillmentType {
    fn from(value: FulfillmentType) -> Self {
        match value {
            FulfillmentType::Digital => Self::Digital,
            FulfillmentType::PhysicalStandard => Self::PhysicalStandard,
        }
    }
}

impl From<ApiFulfillmentType> for FulfillmentType {
    fn from(value: ApiFulfillmentType) -> Self {
        match value {
            ApiFulfillmentType::Digital => Self::Digital,
            ApiFulfillmentType::PhysicalStandard => Self::PhysicalStandard,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiCartItemSource {
    User,
    Ai,
}

impl From<CartItemSource> for ApiCartItemSource {
    fn from(value: CartItemSource) -> Self {
        match value {
            CartItemSource::User => Self::User,
            CartItemSource::Ai => Self::Ai,
        }
    }
}

impl From<ApiCartItemSource> for CartItemSource {
    fn from(value: ApiCartItemSource) -> Self {
        match value {
            ApiCartItemSource::User => Self::User,
            ApiCartItemSource::Ai => Self::Ai,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiProductStatus {
    Draft,
    PendingReview,
    Published,
    Suspended,
    Archived,
}

impl From<ProductStatus> for ApiProductStatus {
    fn from(value: ProductStatus) -> Self {
        match value {
            ProductStatus::Draft => Self::Draft,
            ProductStatus::PendingReview => Self::PendingReview,
            ProductStatus::Published => Self::Published,
            ProductStatus::Suspended => Self::Suspended,
            ProductStatus::Archived => Self::Archived,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiOrderStatus {
    Draft,
    PendingConfirmation,
    PendingPayment,
    PaymentProcessing,
    Paid,
    FulfillmentProcessing,
    Shipped,
    Delivered,
    Completed,
    CancellationRequested,
    Cancelled,
    RefundProcessing,
    Refunded,
    DisputeProcessing,
}

impl From<OrderStatus> for ApiOrderStatus {
    fn from(value: OrderStatus) -> Self {
        match value {
            OrderStatus::Draft => Self::Draft,
            OrderStatus::PendingConfirmation => Self::PendingConfirmation,
            OrderStatus::PendingPayment => Self::PendingPayment,
            OrderStatus::PaymentProcessing => Self::PaymentProcessing,
            OrderStatus::Paid => Self::Paid,
            OrderStatus::FulfillmentProcessing => Self::FulfillmentProcessing,
            OrderStatus::Shipped => Self::Shipped,
            OrderStatus::Delivered => Self::Delivered,
            OrderStatus::Completed => Self::Completed,
            OrderStatus::CancellationRequested => Self::CancellationRequested,
            OrderStatus::Cancelled => Self::Cancelled,
            OrderStatus::RefundProcessing => Self::RefundProcessing,
            OrderStatus::Refunded => Self::Refunded,
            OrderStatus::DisputeProcessing => Self::DisputeProcessing,
        }
    }
}

pub fn actor_user_id(
    headers: &HeaderMap,
    request_id: Uuid,
) -> Result<UserId, (StatusCode, Json<ApiError>)> {
    let Some(header) = headers.get("x-user-id") else {
        return Ok(UserId::from_uuid(DEFAULT_TEST_USER_ID));
    };
    let value = header.to_str().map_err(|_| invalid_user_id(request_id))?;
    let user_id = Uuid::parse_str(value).map_err(|_| invalid_user_id(request_id))?;
    Ok(UserId::from_uuid(user_id))
}

pub fn error_response(
    status: StatusCode,
    code: ErrorCode,
    message: impl Into<String>,
    request_id: Uuid,
) -> (StatusCode, Json<ApiError>) {
    (status, Json(ApiError::new(code, message, request_id)))
}

pub fn not_found(request_id: Uuid, message: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    error_response(
        StatusCode::NOT_FOUND,
        ErrorCode::NotFound,
        message,
        request_id,
    )
}

pub fn invalid_input(request_id: Uuid, message: impl Into<String>) -> (StatusCode, Json<ApiError>) {
    error_response(
        StatusCode::BAD_REQUEST,
        ErrorCode::InvalidInput,
        message,
        request_id,
    )
}

pub fn internal_error(
    request_id: Uuid,
    message: impl Into<String>,
) -> (StatusCode, Json<ApiError>) {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        ErrorCode::InternalError,
        message,
        request_id,
    )
}

fn invalid_user_id(request_id: Uuid) -> (StatusCode, Json<ApiError>) {
    invalid_input(request_id, "X-User-Id must be a UUID")
}
