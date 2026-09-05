use async_trait::async_trait;
use market_bot_shared::{OutboxEvent, OutboxStore, PaymentId};

use crate::domain::{
    payment::{Payment, PaymentError, PaymentHandlingResult},
    refund::{Refund, RefundId},
};

pub struct WebhookApply {
    pub payment: Payment,
    pub refunds: Vec<Refund>,
    pub outbox: Option<OutboxEvent>,
    pub stale: bool,
}

/// Commits `payment_events`, `payments`, refund updates, and `outbox_events`
/// as one unit of work. The event ID is consumed only if that commit succeeds.
#[async_trait]
pub trait PaymentUnitOfWork<O>: Clone + Send + Sync + 'static
where
    O: OutboxStore,
{
    async fn commit_webhook(
        &self,
        outbox: &O,
        event_id: &str,
        payment_id: PaymentId,
        apply: Box<dyn FnOnce(Payment, Vec<Refund>) -> Result<WebhookApply, PaymentError> + Send>,
    ) -> Result<PaymentHandlingResult, PaymentError>;
}

#[async_trait]
pub trait PaymentRepository: Clone + Send + Sync + 'static {
    async fn save_payment(&self, payment: Payment) -> Result<(), PaymentError>;

    async fn find_payment(&self, payment_id: PaymentId) -> Result<Option<Payment>, PaymentError>;

    async fn update_payment(&self, payment: Payment) -> Result<(), PaymentError>;

    /// Returns `true` when the provider event ID was recorded for the first time.
    ///
    /// Safety: webhook retries reuse the same event ID. Recording it before
    /// applying state changes is what makes replay a no-op.
    async fn record_event_id(&self, event_id: &str) -> Result<bool, PaymentError>;

    async fn forget_event_id(&self, event_id: &str) -> Result<(), PaymentError>;

    async fn save_refund(&self, refund: Refund) -> Result<(), PaymentError>;

    async fn find_refund(&self, refund_id: RefundId) -> Result<Option<Refund>, PaymentError>;

    async fn find_refunds_by_payment(
        &self,
        payment_id: PaymentId,
    ) -> Result<Vec<Refund>, PaymentError>;

    async fn update_refund(&self, refund: Refund) -> Result<(), PaymentError>;
}
