use market_bot_shared::{
    CartItemId, CurrencyCode, FulfillmentType, Money, ProductId, ProductVariantId, UserId,
};

use super::{AddCartItem, Cart, CartError, CartItemSource};

fn price(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("price should be valid")
}

fn item(source: CartItemSource, quantity: u64) -> AddCartItem {
    AddCartItem {
        product_id: ProductId::new(),
        variant_id: ProductVariantId::new(),
        title: "Portable Lamp".to_owned(),
        unit_price: price(1_250),
        quantity,
        source,
        fulfillment_type: FulfillmentType::PhysicalStandard,
    }
}

#[test]
fn should_add_an_item_and_preserve_its_source() {
    let mut cart = Cart::new(UserId::new());

    let cart_item = cart
        .add_item(item(CartItemSource::Ai, 2))
        .expect("valid item should be added");

    assert_eq!(cart.items().len(), 1);
    assert_eq!(cart_item.quantity(), 2);
    assert_eq!(cart_item.source(), CartItemSource::Ai);
    assert_eq!(
        cart_item.id(),
        CartItemId::from_uuid(cart_item.id().as_uuid())
    );
}

#[test]
fn should_merge_repeated_items_for_the_same_variant() {
    let variant_id = ProductVariantId::new();
    let mut cart = Cart::new(UserId::new());
    let mut first = item(CartItemSource::User, 1);
    first.variant_id = variant_id;
    let mut second = item(CartItemSource::Ai, 2);
    second.variant_id = variant_id;

    cart.add_item(first).expect("first item should be added");
    cart.add_item(second).expect("same variant should merge");

    assert_eq!(cart.items().len(), 1);
    assert_eq!(cart.items()[0].quantity(), 3);
}

#[test]
fn should_reject_zero_quantity() {
    let mut cart = Cart::new(UserId::new());

    assert_eq!(
        cart.add_item(item(CartItemSource::User, 0)),
        Err(CartError::InvalidQuantity)
    );
}

#[test]
fn should_reject_price_changes_when_merging_an_item() {
    let variant_id = ProductVariantId::new();
    let mut cart = Cart::new(UserId::new());
    let mut first = item(CartItemSource::User, 1);
    first.variant_id = variant_id;
    let mut second = item(CartItemSource::User, 1);
    second.variant_id = variant_id;
    second.unit_price = price(1_500);

    cart.add_item(first).expect("first item should be added");

    assert_eq!(cart.add_item(second), Err(CartError::PriceSnapshotChanged));
}

#[test]
fn should_remove_an_item() {
    let mut cart = Cart::new(UserId::new());
    let cart_item = cart
        .add_item(item(CartItemSource::User, 1))
        .expect("item should be added");

    cart.remove_item(cart_item.id());

    assert!(cart.items().is_empty());
}
