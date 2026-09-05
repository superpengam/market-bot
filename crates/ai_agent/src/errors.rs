use market_bot_cart::CartServiceError;
use market_bot_catalog::CatalogRepositoryError;
use market_bot_order::OrderServiceError;
use market_bot_shared::ErrorCode;

use crate::domain::{AiScope, PolicyReason};
use crate::ports::{AiRepositoryError, CatalogFactsError};

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum AiError {
    #[error("AI scope is unknown")]
    UnknownScope,
    #[error("AI client id cannot be blank")]
    BlankClientId,
    #[error("authorization was not found")]
    AuthorizationNotFound,
    #[error("authorization has expired")]
    AuthorizationExpired,
    #[error("authorization has been revoked")]
    AuthorizationRevoked,
    #[error("missing required AI scope {required}")]
    MissingScope { required: AiScope },
    #[error("user has not enabled auto-purchase")]
    AutoPurchaseDisabled,
    #[error("purchase policy was not found")]
    PolicyNotFound,
    #[error("purchase policy blocked this action: {reason}")]
    PolicyBlocked { reason: PolicyReason },
    #[error("purchase policy requires user confirmation: {reason}")]
    RequiresUserConfirmation { reason: PolicyReason },
    #[error("product was not found")]
    ProductNotFound,
    #[error("product cannot be purchased")]
    ProductNotPurchasable,
    #[error("product is out of stock")]
    ProductOutOfStock,
    #[error("cart was not found")]
    CartNotFound,
    #[error("cart does not belong to the authorized subject")]
    CartOwnerMismatch,
    #[error("quantity must be greater than zero")]
    InvalidQuantity,
    #[error("idempotency key cannot be blank")]
    BlankIdempotencyKey,
    #[error("purchase policy amount cannot be negative")]
    InvalidPolicyAmount,
    #[error("cart operation failed: {0}")]
    Cart(#[source] CartServiceError),
    #[error("order operation failed: {0}")]
    Order(#[source] OrderServiceError),
    #[error("catalog lookup failed: {0}")]
    Catalog(#[source] CatalogRepositoryError),
    #[error("catalog facts lookup failed: {0}")]
    CatalogFacts(#[source] CatalogFactsError),
    #[error("AI repository failed: {0}")]
    Repository(#[source] AiRepositoryError),
}

impl AiError {
    pub const fn error_code(&self) -> ErrorCode {
        match self {
            Self::UnknownScope
            | Self::BlankClientId
            | Self::InvalidQuantity
            | Self::BlankIdempotencyKey
            | Self::InvalidPolicyAmount => ErrorCode::InvalidInput,
            Self::AuthorizationNotFound | Self::ProductNotFound | Self::CartNotFound => {
                ErrorCode::NotFound
            }
            Self::AuthorizationExpired => ErrorCode::AiAuthorizationExpired,
            Self::AuthorizationRevoked
            | Self::MissingScope { .. }
            | Self::AutoPurchaseDisabled
            | Self::CartOwnerMismatch => ErrorCode::Forbidden,
            Self::PolicyNotFound | Self::PolicyBlocked { .. } => {
                ErrorCode::AutoPurchaseLimitExceeded
            }
            Self::RequiresUserConfirmation { .. } => ErrorCode::PriceChanged,
            Self::ProductNotPurchasable => ErrorCode::Forbidden,
            Self::ProductOutOfStock => ErrorCode::ProductOutOfStock,
            Self::Cart(_)
            | Self::Order(_)
            | Self::Catalog(_)
            | Self::CatalogFacts(_)
            | Self::Repository(_) => ErrorCode::InternalError,
        }
    }

    pub const fn audit_code(&self) -> &'static str {
        match self {
            Self::UnknownScope => "UNKNOWN_SCOPE",
            Self::BlankClientId => "BLANK_CLIENT_ID",
            Self::AuthorizationNotFound => "AUTHORIZATION_NOT_FOUND",
            Self::AuthorizationExpired => "AI_AUTHORIZATION_EXPIRED",
            Self::AuthorizationRevoked => "AUTHORIZATION_REVOKED",
            Self::MissingScope { .. } => "MISSING_SCOPE",
            Self::AutoPurchaseDisabled => "AUTO_PURCHASE_DISABLED",
            Self::PolicyNotFound => "POLICY_NOT_FOUND",
            Self::PolicyBlocked { .. } => "AUTO_PURCHASE_LIMIT_EXCEEDED",
            Self::RequiresUserConfirmation { .. } => "REQUIRES_USER_CONFIRMATION",
            Self::ProductNotFound => "NOT_FOUND",
            Self::ProductNotPurchasable => "PRODUCT_NOT_PURCHASABLE",
            Self::ProductOutOfStock => "PRODUCT_OUT_OF_STOCK",
            Self::CartNotFound => "CART_NOT_FOUND",
            Self::CartOwnerMismatch => "CART_OWNER_MISMATCH",
            Self::InvalidQuantity => "INVALID_QUANTITY",
            Self::BlankIdempotencyKey => "BLANK_IDEMPOTENCY_KEY",
            Self::InvalidPolicyAmount => "INVALID_POLICY_AMOUNT",
            Self::Cart(_) => "CART_FAILED",
            Self::Order(_) => "ORDER_FAILED",
            Self::Catalog(_) | Self::CatalogFacts(_) => "CATALOG_FAILED",
            Self::Repository(_) => "REPOSITORY_FAILED",
        }
    }
}
