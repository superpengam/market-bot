use market_bot_shared::{CurrencyCode, Money, ProductId, SellerId};

use super::{Product, ProductError, ProductStatus, ProductType};

#[test]
fn should_create_a_draft_product_with_a_valid_price() {
    let product = Product::new(
        SellerId::new(),
        "Mechanical Keyboard".to_owned(),
        "A standard physical product".to_owned(),
        ProductType::PhysicalStandard,
        Money::new(
            12_999,
            CurrencyCode::try_from("USD").expect("USD should be valid"),
        )
        .expect("price should be valid"),
    )
    .expect("product should be valid");

    assert_eq!(product.status(), ProductStatus::Draft);
    assert_eq!(product.id(), ProductId::from_uuid(product.id().as_uuid()));
}

#[test]
fn should_reject_a_product_with_a_blank_title() {
    let result = Product::new(
        SellerId::new(),
        "   ".to_owned(),
        "Description".to_owned(),
        ProductType::Digital,
        Money::new(
            100,
            CurrencyCode::try_from("USD").expect("USD should be valid"),
        )
        .expect("price should be valid"),
    );

    assert_eq!(result, Err(ProductError::BlankTitle));
}

#[test]
fn should_allow_a_product_to_move_through_review_to_published() {
    let mut product = Product::new(
        SellerId::new(),
        "Downloadable Manual".to_owned(),
        "A digital product".to_owned(),
        ProductType::Digital,
        Money::new(
            500,
            CurrencyCode::try_from("USD").expect("USD should be valid"),
        )
        .expect("price should be valid"),
    )
    .expect("product should be valid");

    product
        .submit_for_review()
        .expect("draft should enter review");
    product.publish().expect("reviewed product should publish");

    assert_eq!(product.status(), ProductStatus::Published);
}

#[test]
fn should_reject_publishing_a_draft_directly() {
    let mut product = Product::new(
        SellerId::new(),
        "Downloadable Manual".to_owned(),
        "A digital product".to_owned(),
        ProductType::Digital,
        Money::new(
            500,
            CurrencyCode::try_from("USD").expect("USD should be valid"),
        )
        .expect("price should be valid"),
    )
    .expect("product should be valid");

    assert_eq!(
        product.publish(),
        Err(ProductError::InvalidStatusTransition)
    );
}

#[test]
fn should_return_a_suspended_product_to_review_then_publish() {
    let mut product = Product::new(
        SellerId::new(),
        "Downloadable Manual".to_owned(),
        "A digital product".to_owned(),
        ProductType::Digital,
        Money::new(
            500,
            CurrencyCode::try_from("USD").expect("USD should be valid"),
        )
        .expect("price should be valid"),
    )
    .expect("product should be valid");

    product
        .submit_for_review()
        .expect("draft should enter review");
    product.publish().expect("reviewed product should publish");
    product.suspend().expect("published product should suspend");

    product
        .return_to_review()
        .expect("suspended product should return to review");
    assert_eq!(product.status(), ProductStatus::PendingReview);

    product
        .publish()
        .expect("returned product should publish again");
    assert_eq!(product.status(), ProductStatus::Published);
}
