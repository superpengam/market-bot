use market_bot_shared::{Money, OrderId, PaymentId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub struct RefundId(Uuid);

impl RefundId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub const fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    pub const fn as_uuid(self) -> Uuid {
        self.0
    }
}

impl Default for RefundId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RefundStatus {
    Requested,
    Processing,
    Succeeded,
    Failed,
}

/// A refund request issued against a succeeded payment.
///
/// Invariant: refund amounts are minor units of the original payment currency
/// and must never exceed the remaining refundable balance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Refund {
    id: RefundId,
    payment_id: PaymentId,
    order_id: OrderId,
    amount: Money,
    status: RefundStatus,
    provider_refund_id: Option<String>,
    reason: String,
}

impl Refund {
    pub fn new(
        payment_id: PaymentId,
        order_id: OrderId,
        amount: Money,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: RefundId::new(),
            payment_id,
            order_id,
            amount,
            status: RefundStatus::Requested,
            provider_refund_id: None,
            reason: reason.into(),
        }
    }

    pub const fn id(&self) -> RefundId {
        self.id
    }

    pub const fn payment_id(&self) -> PaymentId {
        self.payment_id
    }

    pub const fn order_id(&self) -> OrderId {
        self.order_id
    }

    pub const fn amount(&self) -> &Money {
        &self.amount
    }

    pub const fn status(&self) -> RefundStatus {
        self.status
    }

    pub fn provider_refund_id(&self) -> Option<&str> {
        self.provider_refund_id.as_deref()
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn mark_processing(&mut self, provider_refund_id: impl Into<String>) {
        self.status = RefundStatus::Processing;
        self.provider_refund_id = Some(provider_refund_id.into());
    }

    pub fn mark_succeeded(&mut self) {
        self.status = RefundStatus::Succeeded;
    }
}
