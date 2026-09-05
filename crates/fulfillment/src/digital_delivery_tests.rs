use std::sync::Arc;

use chrono::{Duration, Utc};
use market_bot_order::{
    CreateOrderCommand, InMemoryOrderRepository, Order, OrderItem, OrderService, OrderStatus,
};
use market_bot_shared::{
    CurrencyCode, FulfillmentType, Money, ProductId, ProductVariantId, SellerId, UserId,
};

use crate::{
    DigitalAsset, DigitalAssetType, DigitalDeliveryService, FulfillmentError,
    InMemoryDigitalAssetStore, S3ObjectStorage,
};

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

#[test]
fn should_obfuscate_card_secrets_without_storing_plaintext() {
    let asset = DigitalAsset::card_secret(ProductId::new(), "CARD-A");

    assert_eq!(asset.asset_type(), DigitalAssetType::CardSecret);
    assert!(asset.encrypted_reference().starts_with("v1:"));
    assert!(!asset.encrypted_reference().contains("CARD-A"));
    assert_eq!(
        asset
            .reveal_secret()
            .expect("sandbox payload should reverse"),
        "CARD-A"
    );
}

#[tokio::test]
async fn should_issue_an_opaque_expiring_download_url() {
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

    let receipt = delivery_service(orders, assets, storage)
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
    assert!(!download.as_str().contains(object_key));
    assert!(download.expires_at() > Utc::now());
    assert!(
        receipt
            .expires_at()
            .is_some_and(|expires_at| expires_at > Utc::now())
    );
}

#[tokio::test]
async fn should_reuse_the_same_receipt_on_retry() {
    let orders = InMemoryOrderRepository::default();
    let assets = InMemoryDigitalAssetStore::default();
    let storage = S3ObjectStorage::new("https://sandbox-downloads.local");
    let product_id = ProductId::new();
    let order_service = OrderService::new(orders.clone());
    let order = paid_digital_order(&order_service, product_id, SellerId::new()).await;
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
        1
    );
}

#[tokio::test]
async fn should_allocate_distinct_secrets_for_concurrent_orders() {
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
    assert_eq!(
        assets
            .unassigned_count(product_id)
            .await
            .expect("pool should be readable"),
        0
    );
}

#[tokio::test]
async fn should_reject_unpaid_orders() {
    let orders = InMemoryOrderRepository::default();
    let assets = InMemoryDigitalAssetStore::default();
    let storage = S3ObjectStorage::new("https://sandbox-downloads.local");
    let product_id = ProductId::new();
    let order_service = OrderService::new(orders.clone());
    let created = order_service
        .create_order(CreateOrderCommand {
            buyer_id: UserId::new(),
            items: vec![digital_item(product_id, SellerId::new())],
            shipping_fee: usd(0),
            tax: usd(0),
            idempotency_key: "unpaid".to_owned(),
        })
        .await
        .expect("order should be created");
    assets
        .save(DigitalAsset::redeem_code(product_id, "REDEEM-1"))
        .await
        .expect("code should seed");

    let error = delivery_service(orders, assets, storage)
        .fulfill(created.id())
        .await
        .expect_err("draft orders must not be fulfilled");

    assert_eq!(error, FulfillmentError::OrderNotPaid);
}
