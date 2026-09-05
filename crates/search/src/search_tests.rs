use std::collections::BTreeMap;

use market_bot_shared::CurrencyCode;

use super::{InMemorySearchRepository, ProductSearchDocument, SearchProductsQuery, SearchService};

fn published_document(title: &str, stock: u64) -> ProductSearchDocument {
    ProductSearchDocument {
        product_id: market_bot_shared::ProductId::new(),
        variant_ids: vec![market_bot_shared::ProductVariantId::new()],
        title: title.to_owned(),
        searchable_text: title.to_owned(),
        category_ids: Vec::new(),
        attributes: BTreeMap::new(),
        price_minor: 2_500,
        currency: CurrencyCode::try_from("USD").expect("USD should be valid"),
        available_stock: stock,
        is_published: true,
        fulfillment_type: "physical_standard".to_owned(),
    }
}

#[tokio::test]
async fn should_return_only_published_in_stock_matches() {
    let repository = InMemorySearchRepository::default();
    repository
        .insert(published_document("Portable Lamp", 4))
        .await;
    repository.insert(published_document("Desk Mat", 0)).await;

    let service = SearchService::new(repository);
    let results = service
        .search(SearchProductsQuery {
            query: Some("lamp".to_owned()),
            category_id: None,
            currency: Some(CurrencyCode::try_from("USD").expect("USD should be valid")),
            min_price_minor: None,
            max_price_minor: None,
            cursor: None,
        })
        .await
        .expect("search should work");

    assert_eq!(results.items.len(), 1);
    assert_eq!(results.items[0].title, "Portable Lamp");
}

#[tokio::test]
async fn should_exclude_unpublished_documents() {
    let repository = InMemorySearchRepository::default();
    let mut document = published_document("Hidden Lamp", 2);
    document.is_published = false;
    repository.insert(document).await;

    let service = SearchService::new(repository);
    let results = service
        .search(SearchProductsQuery {
            query: None,
            category_id: None,
            currency: None,
            min_price_minor: None,
            max_price_minor: None,
            cursor: None,
        })
        .await
        .expect("search should work");

    assert!(results.items.is_empty());
}

#[tokio::test]
async fn should_filter_by_price_range() {
    let repository = InMemorySearchRepository::default();
    repository
        .insert(published_document("Budget Lamp", 2))
        .await;
    let mut expensive = published_document("Premium Lamp", 2);
    expensive.price_minor = 9_500;
    repository.insert(expensive).await;

    let service = SearchService::new(repository);
    let results = service
        .search(SearchProductsQuery {
            query: Some("lamp".to_owned()),
            category_id: None,
            currency: None,
            min_price_minor: Some(1_000),
            max_price_minor: Some(3_000),
            cursor: None,
        })
        .await
        .expect("search should work");

    assert_eq!(results.items.len(), 1);
    assert_eq!(results.items[0].title, "Budget Lamp");
}
