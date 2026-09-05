use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use market_bot_shared::{CurrencyCode, Money, OrderId, SellerId};
use tokio::sync::Mutex;

use super::{
    CreateSettlementInput, InMemorySettlementStore, PaymentIntent, PaymentIntentInput,
    PaymentProvider, ProviderError, RefundIntent, RefundIntentInput, SettlementBlockReason,
    SettlementError, SettlementIntent, SettlementReleaseInput, SettlementService, SettlementStatus,
    VerifiedPaymentEvent, WebhookHeaders,
};

fn usd(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("amount should be valid")
}

#[derive(Clone, Default)]
struct RecordingSettlementProvider {
    releases: Arc<Mutex<Vec<String>>>,
    release_calls: Arc<AtomicUsize>,
}

#[async_trait]
impl PaymentProvider for RecordingSettlementProvider {
    async fn create_payment_intent(
        &self,
        input: PaymentIntentInput,
    ) -> Result<PaymentIntent, ProviderError> {
        Ok(PaymentIntent::new(
            format!("sandbox_{}", input.order_id),
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

    async fn create_refund(&self, input: RefundIntentInput) -> Result<RefundIntent, ProviderError> {
        Ok(RefundIntent::new(
            format!("sandbox_refund_{}", input.refund_id.as_uuid()),
            input.amount,
        ))
    }

    async fn release_settlement(
        &self,
        input: SettlementReleaseInput,
    ) -> Result<SettlementIntent, ProviderError> {
        self.release_calls.fetch_add(1, Ordering::SeqCst);
        let intent = SettlementIntent::new(
            format!("sandbox_settlement_{}", input.settlement_id.as_uuid()),
            input.amount,
        );
        self.releases
            .lock()
            .await
            .push(intent.provider_settlement_id().to_owned());
        Ok(intent)
    }
}

async fn pending_settlement(
    provider: RecordingSettlementProvider,
) -> (
    SettlementService<InMemorySettlementStore, RecordingSettlementProvider>,
    OrderId,
) {
    let order_id = OrderId::new();
    let service = SettlementService::new(InMemorySettlementStore::default(), provider);
    service
        .create_pending(CreateSettlementInput {
            order_id,
            seller_id: SellerId::new(),
            amount: usd(1_200),
            auto_confirm_at: None,
        })
        .await
        .expect("pending settlement should be created");
    (service, order_id)
}

#[tokio::test]
async fn should_mark_settlement_eligible_after_digital_success() {
    let (service, order_id) = pending_settlement(RecordingSettlementProvider::default()).await;

    service
        .record_digital_delivered(order_id)
        .await
        .expect("digital success should be recorded");
    service
        .mark_eligible(order_id)
        .await
        .expect("digital success should make settlement eligible");

    let stored = service
        .get_by_order(order_id)
        .await
        .expect("settlement lookup should work")
        .expect("settlement should exist");
    let _cloned = stored.clone();

    assert_eq!(stored.status(), SettlementStatus::Eligible);
    assert!(stored.eligible_at().is_some());
}

#[tokio::test]
async fn should_block_settlement_release_when_a_refund_exists() {
    let provider = RecordingSettlementProvider::default();
    let (service, order_id) = pending_settlement(provider.clone()).await;
    service
        .record_digital_delivered(order_id)
        .await
        .expect("digital success should be recorded");
    service
        .record_block(order_id, SettlementBlockReason::Refund)
        .await
        .expect("refund should block settlement");

    let eligible = service.mark_eligible(order_id).await;
    let released = service.release(order_id).await;

    assert!(eligible.is_err());
    assert!(released.is_err());
    assert_eq!(provider.release_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn should_block_settlement_release_when_a_dispute_is_open() {
    let provider = RecordingSettlementProvider::default();
    let (service, order_id) = pending_settlement(provider.clone()).await;
    service
        .record_digital_delivered(order_id)
        .await
        .expect("digital success should be recorded");
    service
        .record_block(order_id, SettlementBlockReason::Dispute)
        .await
        .expect("dispute should block settlement");

    assert!(service.mark_eligible(order_id).await.is_err());
    assert!(service.release(order_id).await.is_err());
    assert_eq!(provider.release_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn should_release_settlement_through_the_payment_provider() {
    let provider = RecordingSettlementProvider::default();
    let (service, order_id) = pending_settlement(provider.clone()).await;
    service
        .record_digital_delivered(order_id)
        .await
        .expect("digital success should be recorded");
    service
        .mark_eligible(order_id)
        .await
        .expect("digital success should make settlement eligible");

    let released = service
        .release(order_id)
        .await
        .expect("eligible settlement should release through the provider");

    assert_eq!(released.status(), SettlementStatus::Released);
    assert!(released.provider_settlement_id().is_some());
    assert_eq!(provider.release_calls.load(Ordering::SeqCst), 1);
    assert_eq!(released.wallet_balance(), None);
    let recorded = provider.releases.lock().await;
    assert_eq!(
        recorded.as_slice(),
        [released
            .provider_settlement_id()
            .expect("provider reference must be stored")
            .to_owned()]
    );
}

#[tokio::test]
async fn should_reject_a_second_pending_settlement_for_the_same_order() {
    let (service, order_id) = pending_settlement(RecordingSettlementProvider::default()).await;

    let duplicate = service
        .create_pending(CreateSettlementInput {
            order_id,
            seller_id: SellerId::new(),
            amount: usd(1_200),
            auto_confirm_at: None,
        })
        .await;

    assert_eq!(duplicate, Err(SettlementError::AlreadyExists));
}

#[tokio::test]
async fn should_reject_release_before_eligibility() {
    let provider = RecordingSettlementProvider::default();
    let (service, order_id) = pending_settlement(provider.clone()).await;
    service
        .record_digital_delivered(order_id)
        .await
        .expect("digital success should be recorded");

    assert_eq!(
        service.release(order_id).await,
        Err(SettlementError::NotEligible)
    );
    assert_eq!(provider.release_calls.load(Ordering::SeqCst), 0);
}
