//! Shared primitives used by Market Bot domain modules.

pub mod errors;
pub mod fulfillment;
pub mod ids;
pub mod money;
pub mod outbox;
pub mod pagination;
pub mod request_context;

#[cfg(test)]
mod error_tests;
#[cfg(test)]
mod money_tests;
#[cfg(test)]
mod outbox_tests;

pub use errors::{ApiError, ErrorCode};
pub use fulfillment::FulfillmentType;
pub use ids::*;
pub use money::{CurrencyCode, CurrencyCodeError, Money, MoneyError};
pub use outbox::{
    InMemoryOutboxStore, OUTBOX_CLAIM_LEASE, OutboxError, OutboxEvent, OutboxStatus, OutboxStore,
    outbox_retry_backoff,
};
pub use pagination::Page;
pub use request_context::{RequestContext, RequestContextError, RequestMetadata};
