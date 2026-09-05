use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    InvalidInput,
    Unauthorized,
    Forbidden,
    NotFound,
    ProductOutOfStock,
    PriceChanged,
    PaymentRequiresAction,
    AiAuthorizationExpired,
    AutoPurchaseLimitExceeded,
    FulfillmentFailed,
    OrderStateInvalid,
    IdempotencyKeyReused,
    InternalError,
}

impl ErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "INVALID_INPUT",
            Self::Unauthorized => "UNAUTHORIZED",
            Self::Forbidden => "FORBIDDEN",
            Self::NotFound => "NOT_FOUND",
            Self::ProductOutOfStock => "PRODUCT_OUT_OF_STOCK",
            Self::PriceChanged => "PRICE_CHANGED",
            Self::PaymentRequiresAction => "PAYMENT_REQUIRES_ACTION",
            Self::AiAuthorizationExpired => "AI_AUTHORIZATION_EXPIRED",
            Self::AutoPurchaseLimitExceeded => "AUTO_PURCHASE_LIMIT_EXCEEDED",
            Self::FulfillmentFailed => "FULFILLMENT_FAILED",
            Self::OrderStateInvalid => "ORDER_STATE_INVALID",
            Self::IdempotencyKeyReused => "IDEMPOTENCY_KEY_REUSED",
            Self::InternalError => "INTERNAL_ERROR",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
    pub request_id: Uuid,
}

impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>, request_id: Uuid) -> Self {
        Self {
            code,
            message: message.into(),
            request_id,
        }
    }

    pub const fn code(&self) -> ErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub const fn request_id(&self) -> Uuid {
        self.request_id
    }
}
