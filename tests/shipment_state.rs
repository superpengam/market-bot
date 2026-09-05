use market_bot_fulfillment::{
    CreateShipmentCommand, LogisticsProvider, SandboxLogisticsProvider, ShipmentService,
    ShipmentStatus, map_logistics_status,
};
use market_bot_order::{
    CreateOrderCommand, InMemoryOrderRepository, Order, OrderItem, OrderService, OrderStatus,
};
use market_bot_shared::{
    CurrencyCode, FulfillmentType, Money, ProductId, ProductVariantId, SellerId, UserId,
};
use market_bot_worker::jobs::sync_shipment_status::SyncShipmentStatusJob;

fn usd(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("amount should be valid")
}

fn physical_item(product_id: ProductId, seller_id: SellerId) -> OrderItem {
    OrderItem::new(
        product_id,
        ProductVariantId::new(),
        seller_id,
        "Mechanical keyboard".to_owned(),
        usd(8_000),
        1,
        FulfillmentType::PhysicalStandard,
    )
    .expect("physical item should be valid")
}

async fn paid_physical_order(orders: &OrderService<InMemoryOrderRepository>) -> Order {
    let created = orders
        .create_order(CreateOrderCommand {
            buyer_id: UserId::new(),
            items: vec![physical_item(ProductId::new(), SellerId::new())],
            shipping_fee: usd(500),
            tax: usd(0),
            idempotency_key: format!("physical-{}", ProductId::new()),
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

#[tokio::test]
async fn should_create_a_shipment_for_a_paid_physical_order_and_mark_it_shipped() {
    let orders = InMemoryOrderRepository::default();
    let order_service = OrderService::new(orders.clone());
    let order = paid_physical_order(&order_service).await;
    let logistics = SandboxLogisticsProvider::new();
    let shipments = ShipmentService::new(orders, logistics);

    let shipment = shipments
        .create_shipment(CreateShipmentCommand {
            order_id: order.id(),
            tracking_number: "TN-SHIP-1".to_owned(),
            carrier: "sandbox".to_owned(),
        })
        .await
        .expect("seller should create a shipment");
    let retry = shipments
        .create_shipment(CreateShipmentCommand {
            order_id: order.id(),
            tracking_number: "TN-SHIP-1".to_owned(),
            carrier: "sandbox".to_owned(),
        })
        .await
        .expect("retry should return the same shipment");
    let reloaded = order_service
        .get_order(order.id())
        .await
        .expect("lookup should work")
        .expect("order should exist");

    assert_eq!(shipment.id(), retry.id());
    assert_eq!(shipment.status(), ShipmentStatus::LabelCreated);
    assert_eq!(reloaded.status(), OrderStatus::Shipped);
}

#[test]
fn should_map_logistics_status_to_platform_states() {
    assert_eq!(
        map_logistics_status("label_created").expect("label"),
        ShipmentStatus::LabelCreated
    );
    assert_eq!(
        map_logistics_status("in_transit").expect("transit"),
        ShipmentStatus::InTransit
    );
    assert_eq!(
        map_logistics_status("delivered").expect("delivered"),
        ShipmentStatus::Delivered
    );
    assert_eq!(
        map_logistics_status("exception").expect("exception"),
        ShipmentStatus::Exception
    );
    assert_eq!(
        map_logistics_status("returned").expect("returned"),
        ShipmentStatus::Returned
    );
}

#[tokio::test]
async fn should_ignore_duplicate_callbacks_that_would_regress_status() {
    let orders = InMemoryOrderRepository::default();
    let order_service = OrderService::new(orders.clone());
    let order = paid_physical_order(&order_service).await;
    let logistics = SandboxLogisticsProvider::new();
    let shipments = ShipmentService::new(orders, logistics.clone());
    let shipment = shipments
        .create_shipment(CreateShipmentCommand {
            order_id: order.id(),
            tracking_number: "TN-SHIP-2".to_owned(),
            carrier: "sandbox".to_owned(),
        })
        .await
        .expect("shipment should be created");

    logistics.set_status("TN-SHIP-2", "in_transit").await;
    let in_transit = shipments
        .sync_status(shipment.id())
        .await
        .expect("in-transit should apply");
    logistics.set_status("TN-SHIP-2", "delivered").await;
    let delivered = shipments
        .sync_status(shipment.id())
        .await
        .expect("delivered should apply");
    logistics.set_status("TN-SHIP-2", "in_transit").await;
    let after_stale = shipments
        .sync_status(shipment.id())
        .await
        .expect("stale in-transit must not regress");
    let after_duplicate = shipments
        .apply_callback("TN-SHIP-2", ShipmentStatus::Delivered)
        .await
        .expect("duplicate delivered is a no-op");

    assert_eq!(in_transit.status(), ShipmentStatus::InTransit);
    assert_eq!(delivered.status(), ShipmentStatus::Delivered);
    assert_eq!(after_stale.status(), ShipmentStatus::Delivered);
    assert_eq!(after_duplicate.status(), ShipmentStatus::Delivered);
    assert!(delivered.is_successful_delivery());
}

#[tokio::test]
async fn should_not_treat_exception_as_successful_delivery() {
    let orders = InMemoryOrderRepository::default();
    let order_service = OrderService::new(orders.clone());
    let order = paid_physical_order(&order_service).await;
    let logistics = SandboxLogisticsProvider::new();
    let shipments = ShipmentService::new(orders, logistics.clone());
    let shipment = shipments
        .create_shipment(CreateShipmentCommand {
            order_id: order.id(),
            tracking_number: "TN-SHIP-3".to_owned(),
            carrier: "sandbox".to_owned(),
        })
        .await
        .expect("shipment should be created");
    logistics.set_status("TN-SHIP-3", "exception").await;
    let job = SyncShipmentStatusJob::new(shipments.clone());

    let updated = job
        .run(shipment.id())
        .await
        .expect("exception should be recorded");
    let mapped = logistics
        .get_tracking_status("TN-SHIP-3")
        .await
        .expect("sandbox status should map");

    assert_eq!(mapped, ShipmentStatus::Exception);
    assert_eq!(updated.status(), ShipmentStatus::Exception);
    assert!(!updated.is_successful_delivery());
    assert!(!ShipmentStatus::Exception.is_successful_delivery());
}
