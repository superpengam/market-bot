use market_bot_shared::{ProductVariantId, SellerId};

use super::{CatalogService, CreateProductCommand, InMemoryCatalogRepository, ProductType};

#[tokio::test]
async fn should_create_and_read_a_product_through_catalog_service() {
    let repository = InMemoryCatalogRepository::default();
    let service = CatalogService::new(repository.clone());
    let product = service
        .create_product(CreateProductCommand {
            seller_id: SellerId::new(),
            title: "Portable Lamp".to_owned(),
            description: "A rechargeable lamp".to_owned(),
            product_type: ProductType::PhysicalStandard,
            price_minor: 4_999,
            currency: "USD".to_owned(),
        })
        .await
        .expect("valid product should be created");

    let loaded = service
        .get_product(product.id())
        .await
        .expect("product should be readable")
        .expect("created product should exist");

    assert_eq!(loaded.id(), product.id());
    assert_eq!(loaded.title(), "Portable Lamp");
}

#[tokio::test]
async fn should_reserve_stock_through_catalog_service() {
    let repository = InMemoryCatalogRepository::default();
    let service = CatalogService::new(repository);
    let variant_id = ProductVariantId::new();
    let reservation_id = market_bot_shared::StockReservationId::new();

    service
        .initialize_inventory(variant_id, 2)
        .await
        .expect("inventory should initialize");
    service
        .reserve_stock(variant_id, 1, reservation_id)
        .await
        .expect("available stock should be reserved");

    let inventory = service
        .get_inventory(variant_id)
        .await
        .expect("inventory should be readable")
        .expect("initialized inventory should exist");

    assert_eq!(inventory.available_stock(), 1);
}
