use chrono::{DateTime, Duration, Utc};
use market_bot_shared::{Money, OrderId, OutboxEvent, OutboxStore};
use serde_json::json;
use uuid::Uuid;

use crate::{
    domain::payment::{
        Payment, PaymentError, PaymentEvent, PaymentEventKind, PaymentHandlingResult, PaymentStatus,
    },
    ports::{
        payment_provider::{PaymentProvider, ProviderError, VerifiedPaymentEvent, WebhookHeaders},
        payment_repository::{PaymentRepository, PaymentUnitOfWork, WebhookApply},
    },
};

const FULFILLMENT_REQUESTED: &str = "fulfillment.requested";

#[derive(Clone)]
pub struct PaymentEventHandler<R, O> {
    repository: R,
    outbox: O,
    max_event_age: Duration,
    max_future_skew: Duration,
}

impl<R, O> PaymentEventHandler<R, O>
where
    R: PaymentRepository + PaymentUnitOfWork<O>,
    O: OutboxStore,
{
    pub fn new(repository: R, outbox: O) -> Self {
        Self {
            repository,
            outbox,
            max_event_age: Duration::seconds(300),
            max_future_skew: Duration::seconds(60),
        }
    }

    pub async fn create_payment(
        &self,
        order_id: OrderId,
        amount: Money,
    ) -> Result<Payment, PaymentError> {
        let payment = Payment::new(order_id, amount);
        self.repository.save_payment(payment.clone()).await?;
        Ok(payment)
    }

    pub async fn handle(
        &self,
        event: VerifiedPaymentEvent,
    ) -> Result<PaymentHandlingResult, PaymentError> {
        self.handle_at(event, Utc::now()).await
    }

    pub async fn handle_at(
        &self,
        event: VerifiedPaymentEvent,
        now: DateTime<Utc>,
    ) -> Result<PaymentHandlingResult, PaymentError> {
        let event = event.into_inner();
        if event.event_id.trim().is_empty() {
            return Err(PaymentError::BlankEventId);
        }

        self.ensure_in_time_window(event.occurred_at, now)?;

        let event_id = event.event_id.clone();
        let payment_id = event.payment_id;
        self.repository
            .commit_webhook(
                &self.outbox,
                &event_id,
                payment_id,
                Box::new(move |payment, refunds| prepare_webhook_apply(&payment, &refunds, &event)),
            )
            .await
    }

    pub async fn handle_webhook<P: PaymentProvider>(
        &self,
        provider: &P,
        headers: &WebhookHeaders,
        body: &[u8],
    ) -> Result<PaymentHandlingResult, PaymentError> {
        let event = provider
            .verify_webhook(headers, body)
            .map_err(map_provider_error)?;
        self.handle(event).await
    }

    fn ensure_in_time_window(
        &self,
        occurred_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<(), PaymentError> {
        if occurred_at > now + self.max_future_skew {
            return Err(PaymentError::EventOutsideTimeWindow);
        }
        if now - occurred_at > self.max_event_age {
            return Err(PaymentError::EventOutsideTimeWindow);
        }
        Ok(())
    }
}

fn prepare_webhook_apply(
    payment: &Payment,
    refunds: &[crate::domain::refund::Refund],
    event: &PaymentEvent,
) -> Result<WebhookApply, PaymentError> {
    if payment.order_id() != event.order_id {
        return Err(PaymentError::OrderMismatch);
    }

    // Invariant: a refund (or any older fact) cannot overwrite a later
    // payment fact that was already applied, even if the webhook arrives last.
    if payment
        .last_fact_occurred_at()
        .is_some_and(|last| event.occurred_at < last)
    {
        return Ok(WebhookApply {
            payment: payment.clone(),
            refunds: Vec::new(),
            outbox: None,
            stale: true,
        });
    }

    let mut payment = payment.clone();
    let should_emit_fulfillment = event.kind == PaymentEventKind::PaymentSucceeded
        && payment.status() != PaymentStatus::Succeeded;
    let mut updated_refunds = Vec::new();

    match event.kind {
        PaymentEventKind::PaymentSucceeded => {
            if payment.status() == PaymentStatus::Created {
                payment.transition_to(PaymentStatus::Processing)?;
            }
            if payment.status() != PaymentStatus::Succeeded {
                payment.transition_to(PaymentStatus::Succeeded)?;
            }
        }
        PaymentEventKind::RefundSucceeded => {
            let amount = event
                .amount
                .clone()
                .ok_or(PaymentError::InvalidRefundAmount)?;
            if amount.minor() <= 0 {
                return Err(PaymentError::InvalidRefundAmount);
            }
            if payment.status() == PaymentStatus::Succeeded {
                payment.transition_to(PaymentStatus::RefundProcessing)?;
            }
            payment.credit_refund(amount)?;
            if payment.is_fully_refunded() {
                payment.transition_to(PaymentStatus::Refunded)?;
            }
            if let Some(mut refund) = refunds
                .iter()
                .find(|refund| {
                    is_open_refund(refund.status()) && refund_matches_event(refund, event)
                })
                .cloned()
            {
                refund.mark_succeeded();
                updated_refunds.push(refund);
            }
        }
    }

    payment.record_fact_time(event.occurred_at);
    let outbox = if should_emit_fulfillment {
        Some(fulfillment_outbox_event(&payment, event))
    } else {
        None
    };

    Ok(WebhookApply {
        payment,
        refunds: updated_refunds,
        outbox,
        stale: false,
    })
}

fn is_open_refund(status: crate::domain::refund::RefundStatus) -> bool {
    matches!(
        status,
        crate::domain::refund::RefundStatus::Requested
            | crate::domain::refund::RefundStatus::Processing
    )
}

fn refund_matches_event(refund: &crate::domain::refund::Refund, event: &PaymentEvent) -> bool {
    if let Some(provider_refund_id) = event.provider_refund_id.as_deref() {
        return refund.provider_refund_id() == Some(provider_refund_id);
    }
    event
        .amount
        .as_ref()
        .is_some_and(|amount| amount == refund.amount())
}

fn fulfillment_outbox_event(payment: &Payment, event: &PaymentEvent) -> OutboxEvent {
    OutboxEvent::new(
        FULFILLMENT_REQUESTED,
        "order",
        payment.order_id().as_uuid(),
        json!({
            "event_id": event.event_id,
            "event_type": FULFILLMENT_REQUESTED,
            "occurred_at": event.occurred_at,
            "aggregate_type": "order",
            "aggregate_id": payment.order_id(),
            "request_id": Uuid::new_v4(),
            "payload_version": 1,
            "order_id": payment.order_id(),
            "payment_id": payment.id(),
        }),
    )
}

pub fn map_provider_error(error: ProviderError) -> PaymentError {
    match error {
        ProviderError::InvalidSignature => PaymentError::InvalidSignature,
        ProviderError::InvalidPayload => PaymentError::InvalidPayload,
        ProviderError::TemporarilyUnavailable => PaymentError::ProviderTemporarilyUnavailable,
    }
}
