use chrono::{DateTime, Utc};
use market_bot_shared::Money;

use crate::{
    application::process_webhook::map_provider_error,
    domain::{
        payment::{PaymentError, PaymentStatus},
        refund::{Refund, RefundStatus},
    },
    ports::{
        payment_provider::{PaymentProvider, RefundIntentInput},
        payment_repository::PaymentRepository,
    },
};

/// Order status values that refund eligibility can inspect without a SQL adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderRefundStatus {
    Paid,
    FulfillmentProcessing,
    Shipped,
    Delivered,
    Completed,
    RefundProcessing,
    Cancelled,
    DisputeProcessing,
    Other,
}

impl OrderRefundStatus {
    pub const fn allows_refund(self) -> bool {
        matches!(
            self,
            Self::Paid
                | Self::FulfillmentProcessing
                | Self::Shipped
                | Self::Delivered
                | Self::Completed
                | Self::RefundProcessing
        )
    }
}

#[derive(Clone, Debug)]
pub struct RequestRefundInput {
    pub payment_id: market_bot_shared::PaymentId,
    pub amount: Money,
    pub reason: String,
    pub order_status: OrderRefundStatus,
    pub after_sale_deadline: DateTime<Utc>,
    pub has_open_dispute: bool,
    pub now: DateTime<Utc>,
}

#[derive(Clone)]
pub struct RefundService<R, P> {
    repository: R,
    provider: P,
}

impl<R, P> RefundService<R, P>
where
    R: PaymentRepository,
    P: PaymentProvider,
{
    pub fn new(repository: R, provider: P) -> Self {
        Self {
            repository,
            provider,
        }
    }

    pub async fn request_refund(&self, input: RequestRefundInput) -> Result<Refund, PaymentError> {
        let reason = input.reason.trim();
        if reason.is_empty() {
            return Err(PaymentError::BlankRefundReason);
        }
        if input.amount.minor() <= 0 {
            return Err(PaymentError::InvalidRefundAmount);
        }
        if !input.order_status.allows_refund() {
            return Err(PaymentError::OrderNotRefundable);
        }
        if input.now > input.after_sale_deadline {
            return Err(PaymentError::AfterSaleWindowClosed);
        }
        if input.has_open_dispute {
            return Err(PaymentError::DisputeOpen);
        }

        let mut payment = self
            .repository
            .find_payment(input.payment_id)
            .await?
            .ok_or(PaymentError::PaymentNotFound)?;

        match payment.status() {
            PaymentStatus::Succeeded | PaymentStatus::RefundProcessing => {}
            _ => return Err(PaymentError::PaymentNotRefundable),
        }

        let refunds = self
            .repository
            .find_refunds_by_payment(input.payment_id)
            .await?;
        let in_flight_minor: i64 = refunds
            .iter()
            .filter(|refund| {
                matches!(
                    refund.status(),
                    RefundStatus::Requested | RefundStatus::Processing
                )
            })
            .map(|refund| refund.amount().minor())
            .sum();
        let reserved = payment
            .refunded_amount()
            .minor()
            .saturating_add(in_flight_minor);
        let remaining = payment.amount().minor().saturating_sub(reserved);
        if input.amount.minor() > remaining {
            return Err(PaymentError::RefundAmountExceedsRemaining);
        }

        let mut refund = Refund::new(
            payment.id(),
            payment.order_id(),
            input.amount.clone(),
            reason,
        );
        self.repository.save_refund(refund.clone()).await?;

        let intent = self
            .provider
            .create_refund(RefundIntentInput {
                payment_id: payment.id(),
                refund_id: refund.id(),
                provider_payment_id: payment.provider_payment_id().to_owned(),
                amount: input.amount.clone(),
                reason: reason.to_owned(),
            })
            .await
            .map_err(map_provider_error)?;

        if payment.status() == PaymentStatus::Succeeded {
            payment.transition_to(PaymentStatus::RefundProcessing)?;
            self.repository.update_payment(payment.clone()).await?;
        }

        refund.mark_processing(intent.provider_refund_id());
        self.repository.update_refund(refund.clone()).await?;
        Ok(refund)
    }
}
