use market_bot_shared::{
    CurrencyCode, FulfillmentType, Money, OrderId, ProductId, ProductVariantId, SellerId, UserId,
};

use super::{Order, OrderError, OrderItem, OrderStatus};

fn price(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("price should be valid")
}

fn order_item(unit_price_minor: i64, quantity: u64) -> OrderItem {
    OrderItem::new(
        ProductId::new(),
        ProductVariantId::new(),
        SellerId::new(),
        "Portable Lamp".to_owned(),
        price(unit_price_minor),
        quantity,
        FulfillmentType::PhysicalStandard,
    )
    .expect("order item should be valid")
}

fn draft_order() -> Order {
    Order::new(
        UserId::new(),
        vec![order_item(1_000, 2)],
        price(500),
        price(150),
    )
    .expect("order should be valid")
}

#[test]
fn should_calculate_subtotal_and_total_from_order_snapshots() {
    let order = draft_order();

    assert_eq!(order.subtotal().minor(), 2_000);
    assert_eq!(order.shipping_fee().minor(), 500);
    assert_eq!(order.tax().minor(), 150);
    assert_eq!(order.total().minor(), 2_650);
}

#[test]
fn should_transition_from_draft_to_pending_payment() {
    let mut order = draft_order();

    order
        .transition_to(OrderStatus::PendingConfirmation)
        .expect("draft should be confirmable");
    order
        .transition_to(OrderStatus::PendingPayment)
        .expect("confirmed order should await payment");

    assert_eq!(order.status(), OrderStatus::PendingPayment);
}

#[test]
fn should_reject_invalid_status_transitions() {
    let mut order = draft_order();

    assert_eq!(
        order.transition_to(OrderStatus::Paid),
        Err(OrderError::InvalidStatusTransition {
            from: OrderStatus::Draft,
            to: OrderStatus::Paid,
        })
    );
}

#[test]
fn should_preserve_order_id_as_a_typed_identifier() {
    let order = draft_order();

    assert_eq!(order.id(), OrderId::from_uuid(order.id().as_uuid()));
}
