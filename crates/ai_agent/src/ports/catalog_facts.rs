use async_trait::async_trait;
use market_bot_shared::{FulfillmentType, Money, ProductId, ProductVariantId, SellerId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogPurchaseFacts {
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub seller_id: SellerId,
    pub title: String,
    pub category_id: String,
    pub seller_score: i32,
    pub unit_price: Money,
    pub shipping_fee: Money,
    pub tax: Money,
    pub available_stock: u64,
    pub fulfillment_type: FulfillmentType,
    pub is_published: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogListingExtras {
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub category_id: String,
    pub seller_score: i32,
    pub shipping_fee: Money,
    pub tax: Money,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum CatalogFactsError {
    #[error("catalog facts storage operation failed")]
    OperationFailed,
}

#[async_trait]
pub trait CatalogFactsReader: Clone + Send + Sync + 'static {
    async fn load_purchase_facts(
        &self,
        product_id: ProductId,
        variant_id: ProductVariantId,
    ) -> Result<Option<CatalogPurchaseFacts>, CatalogFactsError>;

    async fn search_listings(
        &self,
        query: Option<&str>,
        category_id: Option<&str>,
    ) -> Result<Vec<CatalogPurchaseFacts>, CatalogFactsError>;
}
