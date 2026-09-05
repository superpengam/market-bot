use std::collections::HashMap;

use async_trait::async_trait;
use market_bot_shared::{Money, OrderId, PaymentId, SellerId, SettlementId};
use serde::Serialize;

use crate::domain::payment::PaymentEvent;
use crate::domain::refund::RefundId;

pub const SANDBOX_SIGNATURE_HEADER: &str = "x-sandbox-signature";

#[derive(Clone, Debug)]
pub struct PaymentIntentInput {
    pub order_id: OrderId,
    pub amount: Money,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PaymentIntent {
    provider_payment_id: String,
    order_id: OrderId,
    amount: Money,
}

impl PaymentIntent {
    pub fn new(provider_payment_id: String, order_id: OrderId, amount: Money) -> Self {
        Self {
            provider_payment_id,
            order_id,
            amount,
        }
    }

    pub fn provider_payment_id(&self) -> &str {
        &self.provider_payment_id
    }

    pub const fn order_id(&self) -> OrderId {
        self.order_id
    }

    pub const fn amount(&self) -> &Money {
        &self.amount
    }
}

#[derive(Clone, Debug)]
pub struct RefundIntentInput {
    pub payment_id: PaymentId,
    pub refund_id: RefundId,
    pub provider_payment_id: String,
    pub amount: Money,
    pub reason: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct RefundIntent {
    provider_refund_id: String,
    amount: Money,
}

impl RefundIntent {
    pub fn new(provider_refund_id: String, amount: Money) -> Self {
        Self {
            provider_refund_id,
            amount,
        }
    }

    pub fn provider_refund_id(&self) -> &str {
        &self.provider_refund_id
    }

    pub const fn amount(&self) -> &Money {
        &self.amount
    }
}

#[derive(Clone, Debug)]
pub struct SettlementReleaseInput {
    pub settlement_id: SettlementId,
    pub order_id: OrderId,
    pub seller_id: SellerId,
    pub amount: Money,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SettlementIntent {
    provider_settlement_id: String,
    amount: Money,
}

impl SettlementIntent {
    pub fn new(provider_settlement_id: String, amount: Money) -> Self {
        Self {
            provider_settlement_id,
            amount,
        }
    }

    pub fn provider_settlement_id(&self) -> &str {
        &self.provider_settlement_id
    }

    pub const fn amount(&self) -> &Money {
        &self.amount
    }
}

/// HTTP headers forwarded to the provider without taking an HTTP crate dependency.
///
/// Why: the payment domain must stay free of Axum/Hyper types so sandbox and
/// production adapters can share the same verification contract.
#[derive(Clone, Debug, Default)]
pub struct WebhookHeaders {
    values: HashMap<String, String>,
}

impl WebhookHeaders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, name: impl AsRef<str>, value: impl Into<String>) {
        self.values
            .insert(name.as_ref().to_ascii_lowercase(), value.into());
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.values
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedPaymentEvent(pub PaymentEvent);

impl VerifiedPaymentEvent {
    /// Marks an event as signature-verified.
    ///
    /// Safety: production callers must only construct this after
    /// `PaymentProvider::verify_webhook` succeeds.
    pub fn from_verified(event: PaymentEvent) -> Self {
        Self(event)
    }

    pub fn into_inner(self) -> PaymentEvent {
        self.0
    }
}

impl std::ops::Deref for VerifiedPaymentEvent {
    type Target = PaymentEvent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ProviderError {
    #[error("payment webhook signature is invalid")]
    InvalidSignature,
    #[error("payment webhook body is invalid")]
    InvalidPayload,
    #[error("payment provider is temporarily unavailable")]
    TemporarilyUnavailable,
}

#[async_trait]
pub trait PaymentProvider: Clone + Send + Sync + 'static {
    async fn create_payment_intent(
        &self,
        input: PaymentIntentInput,
    ) -> Result<PaymentIntent, ProviderError>;

    fn verify_webhook(
        &self,
        headers: &WebhookHeaders,
        body: &[u8],
    ) -> Result<VerifiedPaymentEvent, ProviderError>;

    async fn create_refund(&self, input: RefundIntentInput) -> Result<RefundIntent, ProviderError>;

    async fn release_settlement(
        &self,
        input: SettlementReleaseInput,
    ) -> Result<SettlementIntent, ProviderError>;
}
