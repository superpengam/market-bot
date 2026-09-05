use market_bot_shared::{
    CurrencyCode, FulfillmentType, Money, OrderId, ProductId, ProductVariantId, SellerId, UserId,
};

use super::{
    CreateOrderCommand, InMemoryOrderRepository, OrderError, OrderItem, OrderService, OrderStatus,
};

fn price(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("price should be valid")
}

fn order_item() -> OrderItem {
    OrderItem::new(
        ProductId::new(),
        ProductVariantId::new(),
        SellerId::new(),
        "Portable Lamp".to_owned(),
        price(1_000),
        2,
        FulfillmentType::PhysicalStandard,
    )
    .expect("order item should be valid")
}

fn command(buyer_id: UserId, key: &str) -> CreateOrderCommand {
    CreateOrderCommand {
        buyer_id,
        items: vec![order_item()],
        shipping_fee: price(500),
        tax: price(150),
        idempotency_key: key.to_owned(),
    }
}

#[tokio::test]
async fn should_create_an_order_once_for_a_repeated_idempotency_key() {
    let service = OrderService::new(InMemoryOrderRepository::default());
    let buyer_id = UserId::new();

    let first = service
        .create_order(command(buyer_id, "checkout-1"))
        .await
        .expect("first order should be created");
    let second = service
        .create_order(command(buyer_id, "checkout-1"))
        .await
        .expect("retry should return the first order");

    assert_eq!(first.id(), second.id());
    assert_eq!(first.total().minor(), 2_650);
}

#[tokio::test]
async fn should_reject_an_empty_idempotency_key() {
    let service = OrderService::new(InMemoryOrderRepository::default());

    let result = service.create_order(command(UserId::new(), "   ")).await;

    assert_eq!(result, Err(super::OrderServiceError::BlankIdempotencyKey));
}

#[tokio::test]
async fn should_transition_an_existing_order_through_the_service() {
    let service = OrderService::new(InMemoryOrderRepository::default());
    let order = service
        .create_order(command(UserId::new(), "checkout-2"))
        .await
        .expect("order should be created");

    let updated = service
        .transition_order(order.id(), OrderStatus::PendingConfirmation)
        .await
        .expect("order should transition");

    assert_eq!(updated.status(), OrderStatus::PendingConfirmation);
}

#[test]
fn should_keep_order_id_type_available_to_application_users() {
    let order_id = OrderId::new();
    assert_eq!(order_id, OrderId::from_uuid(order_id.as_uuid()));
}

#[allow(dead_code)]
fn _order_error_remains_part_of_the_public_domain_contract(_: OrderError) {}
