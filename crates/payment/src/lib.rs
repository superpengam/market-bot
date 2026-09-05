//! Payment and settlement domain module.
//!
//! The platform stores payment-provider references only. Card numbers never
//! enter this module.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod ports;

pub use adapters::in_memory_settlement_store::InMemorySettlementStore;
pub use adapters::in_memory_store::InMemoryPaymentStore;
pub use adapters::sandbox_provider::SandboxPaymentProvider;
pub use application::process_webhook::PaymentEventHandler;
pub use application::release_settlement::{CreateSettlementInput, SettlementService};
pub use application::request_refund::{OrderRefundStatus, RefundService, RequestRefundInput};
pub use domain::payment::{
    Payment, PaymentError, PaymentEvent, PaymentEventKind, PaymentHandlingResult, PaymentStatus,
};
pub use domain::refund::{Refund, RefundId, RefundStatus};
pub use domain::settlement::{
    Settlement, SettlementBlockReason, SettlementError, SettlementStatus,
};
pub use ports::payment_provider::{
    PaymentIntent, PaymentIntentInput, PaymentProvider, ProviderError, RefundIntent,
    RefundIntentInput, SANDBOX_SIGNATURE_HEADER, SettlementIntent, SettlementReleaseInput,
    VerifiedPaymentEvent, WebhookHeaders,
};
pub use ports::payment_repository::{PaymentRepository, PaymentUnitOfWork, WebhookApply};
pub use ports::settlement_store::SettlementStore;

pub type PaymentService<R, O> = PaymentEventHandler<R, O>;
pub type PaymentEventResult = PaymentHandlingResult;

#[cfg(test)]
mod domain_payment_tests;
#[cfg(test)]
mod process_webhook_tests;
#[cfg(test)]
mod provider_tests;
#[cfg(test)]
mod release_settlement_tests;
#[cfg(test)]
mod request_refund_tests;
#[cfg(test)]
mod schema_tests;
