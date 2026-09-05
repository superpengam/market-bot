use market_bot_catalog::{Product, ProductType};
use market_bot_shared::{
    CurrencyCode, FulfillmentType, Money, ProductId, ProductVariantId, SellerId, UserId,
};

use super::{AddCartItem, CartItemSource, CartService, CartServiceError, InMemoryCartRepository};

fn price(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("price should be valid")
}

fn item() -> AddCartItem {
    AddCartItem {
        product_id: ProductId::new(),
        variant_id: ProductVariantId::new(),
        title: "Portable Lamp".to_owned(),
        unit_price: price(1_250),
        quantity: 1,
        source: CartItemSource::Ai,
        fulfillment_type: FulfillmentType::PhysicalStandard,
    }
}

#[tokio::test]
async fn should_create_a_cart_and_add_an_item_through_the_service() {
    let service = CartService::new(InMemoryCartRepository::default());
    let cart = service
        .create_cart(UserId::new())
        .await
        .expect("cart should be created");

    service
        .add_item(cart.id(), item())
        .await
        .expect("item should be added");
    let loaded = service
        .get_cart(cart.id())
        .await
        .expect("cart should be loaded")
        .expect("cart should exist");

    assert_eq!(loaded.items().len(), 1);
}

#[tokio::test]
async fn should_return_not_found_for_an_unknown_cart() {
    let service = CartService::new(InMemoryCartRepository::default());

    let result = service
        .add_item(market_bot_shared::CartId::new(), item())
        .await;

    assert!(matches!(result, Err(super::CartServiceError::CartNotFound)));
}

fn published_product() -> Product {
    let mut product = Product::new(
        SellerId::new(),
        "Portable Lamp".to_owned(),
        "A physical lamp".to_owned(),
        ProductType::PhysicalStandard,
        price(1_250),
    )
    .expect("product should be valid");
    product
        .submit_for_review()
        .expect("draft should enter review");
    product.publish().expect("reviewed product should publish");
    product
}

fn item_for(product: &Product) -> AddCartItem {
    AddCartItem {
        product_id: product.id(),
        variant_id: ProductVariantId::new(),
        title: product.title().to_owned(),
        unit_price: price(1_250),
        quantity: 1,
        source: CartItemSource::User,
        fulfillment_type: FulfillmentType::PhysicalStandard,
    }
}

#[tokio::test]
async fn should_reject_add_purchasable_item_using_product_status() {
    let service = CartService::new(InMemoryCartRepository::default());
    let cart = service
        .create_cart(UserId::new())
        .await
        .expect("cart should be created");
    let mut product = published_product();
    product.suspend().expect("published product should suspend");

    let result = service
        .add_purchasable_item(cart.id(), item_for(&product), &product)
        .await;

    assert!(matches!(
        result,
        Err(CartServiceError::ProductNotPurchasable)
    ));
}

#[tokio::test]
async fn should_add_purchasable_item_when_product_is_published() {
    let service = CartService::new(InMemoryCartRepository::default());
    let cart = service
        .create_cart(UserId::new())
        .await
        .expect("cart should be created");
    let product = published_product();

    service
        .add_purchasable_item(cart.id(), item_for(&product), &product)
        .await
        .expect("published product should be added");
    let loaded = service
        .get_cart(cart.id())
        .await
        .expect("cart should be loaded")
        .expect("cart should exist");

    assert_eq!(loaded.items().len(), 1);
}
