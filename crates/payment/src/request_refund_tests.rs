use async_trait::async_trait;
use chrono::{Duration, Utc};
use market_bot_shared::{CurrencyCode, InMemoryOutboxStore, Money, OrderId};

use super::{
    InMemoryPaymentStore, OrderRefundStatus, PaymentEvent, PaymentEventHandler, PaymentIntent,
    PaymentIntentInput, PaymentProvider, PaymentRepository, PaymentStatus, ProviderError,
    RefundIntent, RefundIntentInput, RefundService, RefundStatus, RequestRefundInput,
    SandboxPaymentProvider, SettlementIntent, SettlementReleaseInput, VerifiedPaymentEvent,
    WebhookHeaders,
};

fn amount(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("amount should be valid")
}

fn refund_input(payment_id: market_bot_shared::PaymentId, refund_minor: i64) -> RequestRefundInput {
    let now = Utc::now();
    RequestRefundInput {
        payment_id,
        amount: amount(refund_minor),
        reason: "buyer changed mind".to_owned(),
        order_status: OrderRefundStatus::Paid,
        after_sale_deadline: now + Duration::days(7),
        has_open_dispute: false,
        now,
    }
}

async fn succeeded_payment() -> (
    PaymentEventHandler<InMemoryPaymentStore, InMemoryOutboxStore>,
    InMemoryPaymentStore,
    super::Payment,
) {
    let store = InMemoryPaymentStore::default();
    let handler = PaymentEventHandler::new(store.clone(), InMemoryOutboxStore::default());
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    handler
        .handle(VerifiedPaymentEvent::from_verified(
            PaymentEvent::succeeded("provider-event-paid", payment.id(), payment.order_id()),
        ))
        .await
        .expect("payment should succeed");
    (handler, store, payment)
}

#[tokio::test]
async fn should_request_a_refund_when_order_checks_pass() {
    let (_handler, store, payment) = succeeded_payment().await;
    let service = RefundService::new(store, SandboxPaymentProvider::new("test-secret"));

    let refund = service
        .request_refund(refund_input(payment.id(), 1_500))
        .await
        .expect("refund should be requested");

    assert_eq!(refund.status(), RefundStatus::Processing);
    assert!(refund.provider_refund_id().is_some());
}

#[tokio::test]
async fn should_move_a_succeeded_payment_into_refund_processing() {
    let (_handler, store, payment) = succeeded_payment().await;
    let service = RefundService::new(store.clone(), SandboxPaymentProvider::new("test-secret"));

    service
        .request_refund(refund_input(payment.id(), 500))
        .await
        .expect("refund should begin");

    let stored = store
        .find_payment(payment.id())
        .await
        .expect("lookup should work")
        .expect("payment should exist");
    assert_eq!(stored.status(), PaymentStatus::RefundProcessing);
}

#[tokio::test]
async fn should_reject_a_refund_when_the_after_sale_window_closed() {
    let (_handler, store, payment) = succeeded_payment().await;
    let service = RefundService::new(store, SandboxPaymentProvider::new("test-secret"));
    let now = Utc::now();
    let mut input = refund_input(payment.id(), 1_500);
    input.now = now;
    input.after_sale_deadline = now - Duration::seconds(1);

    assert_eq!(
        service.request_refund(input).await,
        Err(super::PaymentError::AfterSaleWindowClosed)
    );
}

#[tokio::test]
async fn should_reject_a_refund_when_the_amount_exceeds_the_remaining_balance() {
    let (_handler, store, payment) = succeeded_payment().await;
    let service = RefundService::new(store, SandboxPaymentProvider::new("test-secret"));

    service
        .request_refund(refund_input(payment.id(), 800))
        .await
        .expect("first refund should reserve part of the balance");

    assert_eq!(
        service
            .request_refund(refund_input(payment.id(), 1_000))
            .await,
        Err(super::PaymentError::RefundAmountExceedsRemaining)
    );
}

#[tokio::test]
async fn should_reject_a_second_refund_when_caller_passes_zero_already_refunded() {
    let (_handler, store, payment) = succeeded_payment().await;
    let service = RefundService::new(store, SandboxPaymentProvider::new("test-secret"));

    service
        .request_refund(refund_input(payment.id(), 1_500))
        .await
        .expect("first refund should take the full balance");

    assert_eq!(
        service
            .request_refund(refund_input(payment.id(), 100))
            .await,
        Err(super::PaymentError::RefundAmountExceedsRemaining)
    );
}

#[derive(Clone)]
struct FailingRefundProvider;

#[async_trait]
impl PaymentProvider for FailingRefundProvider {
    async fn create_payment_intent(
        &self,
        input: PaymentIntentInput,
    ) -> Result<PaymentIntent, ProviderError> {
        Ok(PaymentIntent::new(
            "unused".to_owned(),
            input.order_id,
            input.amount,
        ))
    }

    fn verify_webhook(
        &self,
        _headers: &WebhookHeaders,
        _body: &[u8],
    ) -> Result<VerifiedPaymentEvent, ProviderError> {
        Err(ProviderError::InvalidSignature)
    }

    async fn create_refund(
        &self,
        _input: RefundIntentInput,
    ) -> Result<RefundIntent, ProviderError> {
        Err(ProviderError::TemporarilyUnavailable)
    }

    async fn release_settlement(
        &self,
        _input: SettlementReleaseInput,
    ) -> Result<SettlementIntent, ProviderError> {
        Err(ProviderError::TemporarilyUnavailable)
    }
}

#[tokio::test]
async fn should_keep_a_requested_refund_when_the_provider_fails() {
    let (_handler, store, payment) = succeeded_payment().await;
    let service = RefundService::new(store.clone(), FailingRefundProvider);

    assert!(
        service
            .request_refund(refund_input(payment.id(), 500))
            .await
            .is_err()
    );

    let refunds = store
        .find_refunds_by_payment(payment.id())
        .await
        .expect("refunds should load");
    assert_eq!(refunds.len(), 1);
    assert_eq!(refunds[0].status(), RefundStatus::Requested);
    assert!(refunds[0].provider_refund_id().is_none());
}

#[tokio::test]
async fn should_reject_a_refund_when_a_dispute_is_open() {
    let (_handler, store, payment) = succeeded_payment().await;
    let service = RefundService::new(store, SandboxPaymentProvider::new("test-secret"));
    let mut input = refund_input(payment.id(), 1_500);
    input.has_open_dispute = true;

    assert_eq!(
        service.request_refund(input).await,
        Err(super::PaymentError::DisputeOpen)
    );
}

#[tokio::test]
async fn should_reject_a_refund_when_the_order_status_is_not_refundable() {
    let (_handler, store, payment) = succeeded_payment().await;
    let service = RefundService::new(store, SandboxPaymentProvider::new("test-secret"));
    let mut input = refund_input(payment.id(), 1_500);
    input.order_status = OrderRefundStatus::Cancelled;

    assert_eq!(
        service.request_refund(input).await,
        Err(super::PaymentError::OrderNotRefundable)
    );
}
