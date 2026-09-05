use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use market_bot_fulfillment::{
    DigitalAsset, DigitalAssetType, DigitalDeliveryService, InMemoryDigitalAssetStore,
    S3ObjectStorage,
};
use market_bot_order::{
    CreateOrderCommand, InMemoryOrderRepository, Order, OrderItem, OrderService, OrderStatus,
};
use market_bot_payment::{
    CreateSettlementInput, InMemorySettlementStore, PaymentIntent, PaymentIntentInput,
    PaymentProvider, ProviderError, RefundIntent, RefundIntentInput, SettlementBlockReason,
    SettlementReleaseInput, SettlementService, SettlementStatus, VerifiedPaymentEvent,
    WebhookHeaders,
};
use market_bot_shared::{
    CurrencyCode, FulfillmentType, Money, ProductId, ProductVariantId, SellerId, UserId,
};
use market_bot_worker::jobs::fulfill_digital_order::FulfillDigitalOrderJob;
use tokio::sync::Mutex;

fn usd(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("amount should be valid")
}

fn digital_item(product_id: ProductId, seller_id: SellerId) -> OrderItem {
    OrderItem::new(
        product_id,
        ProductVariantId::new(),
        seller_id,
        "Season pass".to_owned(),
        usd(1_200),
        1,
        FulfillmentType::Digital,
    )
    .expect("digital item should be valid")
}

async fn paid_digital_order(
    orders: &OrderService<InMemoryOrderRepository>,
    product_id: ProductId,
    seller_id: SellerId,
) -> Order {
    let created = orders
        .create_order(CreateOrderCommand {
            buyer_id: UserId::new(),
            items: vec![digital_item(product_id, seller_id)],
            shipping_fee: usd(0),
            tax: usd(0),
            idempotency_key: format!("digital-{}", ProductId::new()),
        })
        .await
        .expect("order should be created");

    for status in [
        OrderStatus::PendingConfirmation,
        OrderStatus::PendingPayment,
        OrderStatus::PaymentProcessing,
        OrderStatus::Paid,
    ] {
        orders
            .transition_order(created.id(), status)
            .await
            .expect("paid path should be valid");
    }

    orders
        .get_order(created.id())
        .await
        .expect("lookup should work")
        .expect("order should exist")
}

fn delivery_service(
    orders: InMemoryOrderRepository,
    assets: InMemoryDigitalAssetStore,
    storage: S3ObjectStorage,
) -> DigitalDeliveryService<InMemoryOrderRepository, InMemoryDigitalAssetStore, S3ObjectStorage> {
    DigitalDeliveryService::new(orders, assets, storage, Duration::hours(1))
}

#[tokio::test]
async fn should_issue_a_digital_delivery_receipt_exactly_once_for_a_paid_order() {
    let orders = InMemoryOrderRepository::default();
    let assets = InMemoryDigitalAssetStore::default();
    let storage = S3ObjectStorage::new("https://sandbox-downloads.local");
    let product_id = ProductId::new();
    let seller_id = SellerId::new();
    let order_service = OrderService::new(orders.clone());
    let order = paid_digital_order(&order_service, product_id, seller_id).await;
    let object_key = "products/season-pass/build.bin";
    let asset = DigitalAsset::file(product_id, object_key);
    assets
        .save(asset.clone())
        .await
        .expect("file asset should seed");
    storage
        .put_object(asset.id(), object_key)
        .await
        .expect("object should be stored privately");
    let service = delivery_service(orders, assets, storage);

    let first = service
        .fulfill(order.id())
        .await
        .expect("paid digital order should be fulfilled");
    let second = service
        .fulfill(order.id())
        .await
        .expect("retry should return the original receipt");

    assert_eq!(first.fulfillment_id(), second.fulfillment_id());
    assert_eq!(first.order_id(), order.id());
    let download = first
        .download_url()
        .expect("file delivery should include a download URL");
    assert!(download.expires_at() > Utc::now());
}

#[tokio::test]
async fn should_not_reissue_a_one_time_credential_on_retry() {
    let orders = InMemoryOrderRepository::default();
    let assets = InMemoryDigitalAssetStore::default();
    let storage = S3ObjectStorage::new("https://sandbox-downloads.local");
    let product_id = ProductId::new();
    let seller_id = SellerId::new();
    let order_service = OrderService::new(orders.clone());
    let order = paid_digital_order(&order_service, product_id, seller_id).await;
    assets
        .save(DigitalAsset::card_secret(product_id, "CARD-SECRET-ONE"))
        .await
        .expect("first card should seed");
    assets
        .save(DigitalAsset::card_secret(product_id, "CARD-SECRET-TWO"))
        .await
        .expect("second card should seed");
    let service = delivery_service(orders, assets.clone(), storage);

    let first = service
        .fulfill(order.id())
        .await
        .expect("first fulfill should allocate one secret");
    let second = service
        .fulfill(order.id())
        .await
        .expect("retry must reuse the allocated secret");

    assert_eq!(first.fulfillment_id(), second.fulfillment_id());
    assert_eq!(first.revealed_secret(), second.revealed_secret());
    assert_eq!(
        assets
            .unassigned_count(product_id)
            .await
            .expect("pool should be readable"),
        1,
        "retry must not consume a second one-time credential"
    );
}

#[tokio::test]
async fn should_return_an_expiring_download_url_instead_of_the_raw_storage_path() {
    let orders = InMemoryOrderRepository::default();
    let assets = InMemoryDigitalAssetStore::default();
    let storage = S3ObjectStorage::new("https://sandbox-downloads.local");
    let product_id = ProductId::new();
    let object_key = "private-bucket/never-expose/game.iso";
    let order_service = OrderService::new(orders.clone());
    let order = paid_digital_order(&order_service, product_id, SellerId::new()).await;
    let asset = DigitalAsset::file(product_id, object_key);
    assets.save(asset.clone()).await.expect("asset should seed");
    storage
        .put_object(asset.id(), object_key)
        .await
        .expect("object should be stored privately");
    let service = delivery_service(orders, assets, storage);

    let receipt = service
        .fulfill(order.id())
        .await
        .expect("file should be delivered");
    let download = receipt
        .download_url()
        .expect("file delivery should mint a download URL");

    assert!(
        download
            .as_str()
            .starts_with("https://sandbox-downloads.local/")
    );
    assert!(
        !download.as_str().contains(object_key),
        "clients must never see the raw object-storage path"
    );
    assert!(download.expires_at() > Utc::now());
    assert!(
        receipt
            .expires_at()
            .is_some_and(|expires_at| expires_at > Utc::now())
    );
}

#[tokio::test]
async fn should_allocate_a_one_time_card_secret_atomically_from_the_unassigned_pool() {
    let orders = InMemoryOrderRepository::default();
    let assets = InMemoryDigitalAssetStore::default();
    let storage = S3ObjectStorage::new("https://sandbox-downloads.local");
    let product_id = ProductId::new();
    let seller_id = SellerId::new();
    let order_service = OrderService::new(orders.clone());
    let first_order = paid_digital_order(&order_service, product_id, seller_id).await;
    let second_order = paid_digital_order(&order_service, product_id, seller_id).await;
    assets
        .save(DigitalAsset::card_secret(product_id, "CARD-A"))
        .await
        .expect("first card should seed");
    assets
        .save(DigitalAsset::card_secret(product_id, "CARD-B"))
        .await
        .expect("second card should seed");
    let service = Arc::new(delivery_service(orders, assets.clone(), storage));

    let service_a = Arc::clone(&service);
    let service_b = Arc::clone(&service);
    let first_id = first_order.id();
    let second_id = second_order.id();
    let (first, second) = tokio::join!(
        async move { service_a.fulfill(first_id).await },
        async move { service_b.fulfill(second_id).await }
    );
    let first = first.expect("first order should receive a secret");
    let second = second.expect("second order should receive a secret");

    assert_ne!(first.revealed_secret(), second.revealed_secret());
    assert!(matches!(
        first.revealed_secret(),
        Some("CARD-A") | Some("CARD-B")
    ));
    assert_eq!(
        assets
            .unassigned_count(product_id)
            .await
            .expect("pool should be readable"),
        0
    );
    let assigned = assets
        .find_assigned_to_order(first_order.id())
        .await
        .expect("assignment lookup should work")
        .expect("first order should own a card");
    assert_eq!(assigned.asset_type(), DigitalAssetType::CardSecret);
    assert!(
        !assigned.encrypted_reference().contains("CARD-A")
            && !assigned.encrypted_reference().contains("CARD-B"),
        "card secrets must be stored encrypted, not as plaintext"
    );
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
    ) -> Result<market_bot_payment::SettlementIntent, ProviderError> {
        self.release_calls.fetch_add(1, Ordering::SeqCst);
        let intent = market_bot_payment::SettlementIntent::new(
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
    store: InMemorySettlementStore,
    provider: RecordingSettlementProvider,
    order: &Order,
) -> SettlementService<InMemorySettlementStore, RecordingSettlementProvider> {
    let service = SettlementService::new(store, provider);
    service
        .create_pending(CreateSettlementInput {
            order_id: order.id(),
            seller_id: order.items()[0].seller_id(),
            amount: order.total().clone(),
            auto_confirm_at: None,
        })
        .await
        .expect("pending settlement should be created");
    service
}

#[tokio::test]
async fn should_mark_settlement_eligible_after_digital_success() {
    let orders = InMemoryOrderRepository::default();
    let assets = InMemoryDigitalAssetStore::default();
    let storage = S3ObjectStorage::new("https://sandbox-downloads.local");
    let settlements = InMemorySettlementStore::default();
    let provider = RecordingSettlementProvider::default();
    let product_id = ProductId::new();
    let seller_id = SellerId::new();
    let order_service = OrderService::new(orders.clone());
    let order = paid_digital_order(&order_service, product_id, seller_id).await;
    assets
        .save(DigitalAsset::redeem_code(product_id, "REDEEM-1"))
        .await
        .expect("redeem code should seed");
    let delivery = delivery_service(orders, assets, storage);
    let settlement = pending_settlement(settlements, provider, &order).await;
    let job = FulfillDigitalOrderJob::new(delivery, settlement.clone());

    job.run(order.id())
        .await
        .expect("digital job should fulfill and request settlement");
    let stored = settlement
        .get_by_order(order.id())
        .await
        .expect("settlement lookup should work")
        .expect("settlement should exist");

    assert_eq!(stored.status(), SettlementStatus::Eligible);
    assert!(stored.eligible_at().is_some());
}

#[tokio::test]
async fn should_block_settlement_release_when_a_refund_exists() {
    let orders = InMemoryOrderRepository::default();
    let order_service = OrderService::new(orders);
    let order = paid_digital_order(&order_service, ProductId::new(), SellerId::new()).await;
    let provider = RecordingSettlementProvider::default();
    let settlement =
        pending_settlement(InMemorySettlementStore::default(), provider.clone(), &order).await;
    settlement
        .record_digital_delivered(order.id())
        .await
        .expect("digital success should be recorded");
    settlement
        .record_block(order.id(), SettlementBlockReason::Refund)
        .await
        .expect("refund should block settlement");

    let eligible = settlement.mark_eligible(order.id()).await;
    let released = settlement.release(order.id()).await;

    assert!(eligible.is_err());
    assert!(released.is_err());
    assert_eq!(provider.release_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn should_block_settlement_release_when_a_dispute_is_open() {
    let orders = InMemoryOrderRepository::default();
    let order_service = OrderService::new(orders);
    let order = paid_digital_order(&order_service, ProductId::new(), SellerId::new()).await;
    let provider = RecordingSettlementProvider::default();
    let settlement =
        pending_settlement(InMemorySettlementStore::default(), provider.clone(), &order).await;
    settlement
        .record_digital_delivered(order.id())
        .await
        .expect("digital success should be recorded");
    settlement
        .record_block(order.id(), SettlementBlockReason::Dispute)
        .await
        .expect("dispute should block settlement");

    assert!(settlement.mark_eligible(order.id()).await.is_err());
    assert!(settlement.release(order.id()).await.is_err());
    assert_eq!(provider.release_calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn should_release_settlement_through_the_payment_provider() {
    let orders = InMemoryOrderRepository::default();
    let order_service = OrderService::new(orders);
    let order = paid_digital_order(&order_service, ProductId::new(), SellerId::new()).await;
    let provider = RecordingSettlementProvider::default();
    let settlement =
        pending_settlement(InMemorySettlementStore::default(), provider.clone(), &order).await;
    settlement
        .record_digital_delivered(order.id())
        .await
        .expect("digital success should be recorded");
    settlement
        .mark_eligible(order.id())
        .await
        .expect("digital success should make settlement eligible");

    let released = settlement
        .release(order.id())
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
