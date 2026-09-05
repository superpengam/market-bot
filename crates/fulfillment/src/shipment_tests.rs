use market_bot_order::{
    CreateOrderCommand, InMemoryOrderRepository, Order, OrderItem, OrderService, OrderStatus,
};
use market_bot_shared::{
    CurrencyCode, FulfillmentType, Money, OrderId, ProductId, ProductVariantId, SellerId, UserId,
};

use crate::{
    CreateShipmentCommand, SandboxLogisticsProvider, Shipment, ShipmentService, ShipmentStatus,
    map_logistics_status,
};

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
        "Keyboard".to_owned(),
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

#[test]
fn should_map_carrier_status_strings_to_platform_states() {
    assert_eq!(
        map_logistics_status("label_created").expect("label"),
        ShipmentStatus::LabelCreated
    );
    assert_eq!(
        map_logistics_status("in-transit").expect("transit"),
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

#[test]
fn should_not_regress_shipment_status_on_stale_callbacks() {
    let mut shipment =
        Shipment::new(OrderId::new(), "TN-1", "sandbox").expect("shipment should be valid");

    shipment.apply_status(ShipmentStatus::InTransit);
    shipment.apply_status(ShipmentStatus::Delivered);
    shipment.apply_status(ShipmentStatus::InTransit);
    shipment.apply_status(ShipmentStatus::Delivered);

    assert_eq!(shipment.status(), ShipmentStatus::Delivered);
    assert!(shipment.is_successful_delivery());
}

#[test]
fn should_not_treat_exception_as_successful_delivery() {
    let mut shipment =
        Shipment::new(OrderId::new(), "TN-2", "sandbox").expect("shipment should be valid");
    shipment.apply_status(ShipmentStatus::InTransit);
    shipment.apply_status(ShipmentStatus::Exception);
    shipment.apply_status(ShipmentStatus::InTransit);

    assert_eq!(shipment.status(), ShipmentStatus::Exception);
    assert!(!shipment.is_successful_delivery());
    assert!(!ShipmentStatus::Exception.is_successful_delivery());
}

#[tokio::test]
async fn should_mark_a_paid_physical_order_shipped_when_creating_a_shipment() {
    let orders = InMemoryOrderRepository::default();
    let order_service = OrderService::new(orders.clone());
    let order = paid_physical_order(&order_service).await;
    let logistics = SandboxLogisticsProvider::new();
    let shipments = ShipmentService::new(orders, logistics);

    let shipment = shipments
        .create_shipment(CreateShipmentCommand {
            order_id: order.id(),
            tracking_number: "TN-SHIP".to_owned(),
            carrier: "sandbox".to_owned(),
        })
        .await
        .expect("seller should create a shipment");
    let reloaded = order_service
        .get_order(order.id())
        .await
        .expect("lookup should work")
        .expect("order should exist");

    assert_eq!(shipment.status(), ShipmentStatus::LabelCreated);
    assert_eq!(reloaded.status(), OrderStatus::Shipped);
}
