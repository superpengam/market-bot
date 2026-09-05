use chrono::{DateTime, Utc};
use market_bot_shared::{Money, OrderId, PaymentId};
use serde::{Deserialize, Serialize};

/// A payment tracked by the platform.
///
/// Safety: only provider references are stored. Card numbers, PAN, and CVV
/// must never be added to this entity or its persistence model.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Payment {
    id: PaymentId,
    order_id: OrderId,
    amount: Money,
    status: PaymentStatus,
    provider_name: String,
    provider_payment_id: String,
    refunded_amount: Money,
    last_fact_occurred_at: Option<DateTime<Utc>>,
}

impl Payment {
    pub fn new(order_id: OrderId, amount: Money) -> Self {
        let refunded_amount =
            Money::new(0, amount.currency().clone()).expect("zero is a valid money amount");
        Self {
            id: PaymentId::new(),
            order_id,
            amount,
            status: PaymentStatus::Created,
            provider_name: "sandbox".to_owned(),
            provider_payment_id: format!("sandbox_{order_id}"),
            refunded_amount,
            last_fact_occurred_at: None,
        }
    }

    pub const fn id(&self) -> PaymentId {
        self.id
    }

    pub const fn order_id(&self) -> OrderId {
        self.order_id
    }

    pub const fn amount(&self) -> &Money {
        &self.amount
    }

    pub const fn status(&self) -> PaymentStatus {
        self.status
    }

    pub fn provider_name(&self) -> &str {
        &self.provider_name
    }

    pub fn provider_payment_id(&self) -> &str {
        &self.provider_payment_id
    }

    pub const fn refunded_amount(&self) -> &Money {
        &self.refunded_amount
    }

    pub const fn last_fact_occurred_at(&self) -> Option<DateTime<Utc>> {
        self.last_fact_occurred_at
    }

    pub fn transition_to(&mut self, next: PaymentStatus) -> Result<(), PaymentError> {
        if !self.status.can_transition_to(next) {
            return Err(PaymentError::InvalidStatusTransition {
                from: self.status,
                to: next,
            });
        }

        self.status = next;
        Ok(())
    }

    /// Invariant: a later provider fact is the source of truth. Older events
    /// may be recorded for idempotency but must not move this timestamp backward.
    pub fn record_fact_time(&mut self, occurred_at: DateTime<Utc>) {
        match self.last_fact_occurred_at {
            Some(previous) if occurred_at < previous => {}
            _ => self.last_fact_occurred_at = Some(occurred_at),
        }
    }

    pub fn add_refunded_amount(&mut self, amount: Money) -> Result<(), PaymentError> {
        self.refunded_amount = self
            .refunded_amount
            .clone()
            .checked_add(amount)
            .map_err(PaymentError::Money)?;
        Ok(())
    }

    pub fn remaining_refundable(&self) -> Result<Money, PaymentError> {
        self.amount
            .clone()
            .checked_sub(self.refunded_amount.clone())
            .map_err(PaymentError::Money)
    }

    /// Credits a refund callback, never letting `refunded_amount` exceed the payment total.
    pub fn credit_refund(&mut self, amount: Money) -> Result<Money, PaymentError> {
        let remaining = self.remaining_refundable()?;
        let applied_minor = amount.minor().min(remaining.minor());
        let applied =
            Money::new(applied_minor, amount.currency().clone()).map_err(PaymentError::Money)?;
        if applied.minor() > 0 {
            self.add_refunded_amount(applied.clone())?;
        }
        Ok(applied)
    }

    pub fn is_fully_refunded(&self) -> bool {
        self.refunded_amount.minor() >= self.amount.minor()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PaymentStatus {
    Created,
    Processing,
    Succeeded,
    Failed,
    RefundProcessing,
    Refunded,
}

impl PaymentStatus {
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Created, Self::Processing | Self::Failed)
                | (Self::Processing, Self::Succeeded | Self::Failed)
                | (Self::Succeeded, Self::RefundProcessing | Self::Refunded)
                | (Self::RefundProcessing, Self::Refunded)
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PaymentEvent {
    pub event_id: String,
    pub payment_id: PaymentId,
    pub order_id: OrderId,
    pub kind: PaymentEventKind,
    pub occurred_at: DateTime<Utc>,
    #[serde(default)]
    pub amount: Option<Money>,
    #[serde(default)]
    pub provider_refund_id: Option<String>,
}

impl PaymentEvent {
    pub fn succeeded(
        event_id: impl Into<String>,
        payment_id: PaymentId,
        order_id: OrderId,
    ) -> Self {
        Self::succeeded_at(event_id, payment_id, order_id, Utc::now())
    }

    pub fn succeeded_at(
        event_id: impl Into<String>,
        payment_id: PaymentId,
        order_id: OrderId,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            payment_id,
            order_id,
            kind: PaymentEventKind::PaymentSucceeded,
            occurred_at,
            amount: None,
            provider_refund_id: None,
        }
    }

    pub fn refund_succeeded_at(
        event_id: impl Into<String>,
        payment_id: PaymentId,
        order_id: OrderId,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            payment_id,
            order_id,
            kind: PaymentEventKind::RefundSucceeded,
            occurred_at,
            amount: None,
            provider_refund_id: None,
        }
    }

    pub fn refund_succeeded_with_amount(
        event_id: impl Into<String>,
        payment_id: PaymentId,
        order_id: OrderId,
        occurred_at: DateTime<Utc>,
        amount: Money,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            payment_id,
            order_id,
            kind: PaymentEventKind::RefundSucceeded,
            occurred_at,
            amount: Some(amount),
            provider_refund_id: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PaymentEventKind {
    PaymentSucceeded,
    RefundSucceeded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaymentHandlingResult {
    Applied,
    Duplicate,
    IgnoredStale,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum PaymentError {
    #[error("payment was not found")]
    PaymentNotFound,
    #[error("refund was not found")]
    RefundNotFound,
    #[error("payment event ID cannot be blank")]
    BlankEventId,
    #[error("payment event order does not match payment order")]
    OrderMismatch,
    #[error("payment webhook signature is invalid")]
    InvalidSignature,
    #[error("payment webhook body is invalid")]
    InvalidPayload,
    #[error("payment event is outside the accepted time window")]
    EventOutsideTimeWindow,
    #[error("payment status transition from {from:?} to {to:?} is invalid")]
    InvalidStatusTransition {
        from: PaymentStatus,
        to: PaymentStatus,
    },
    #[error("refund is not allowed for the current order status")]
    OrderNotRefundable,
    #[error("after-sale window has closed")]
    AfterSaleWindowClosed,
    #[error("refund amount exceeds the remaining refundable amount")]
    RefundAmountExceedsRemaining,
    #[error("refund is blocked because the order has an open dispute")]
    DisputeOpen,
    #[error("refund reason cannot be blank")]
    BlankRefundReason,
    #[error("refund amount must be greater than zero")]
    InvalidRefundAmount,
    #[error("payment is not in a refundable status")]
    PaymentNotRefundable,
    #[error("payment money operation failed: {0}")]
    Money(#[source] market_bot_shared::MoneyError),
    #[error("payment provider is temporarily unavailable")]
    ProviderTemporarilyUnavailable,
    #[error("payment provider request failed")]
    ProviderFailed,
    #[error("outbox operation failed: {0}")]
    Outbox(#[source] market_bot_shared::OutboxError),
}
