use async_trait::async_trait;
use market_bot_shared::{ProductId, ProductVariantId, StockReservationId};

use crate::{Inventory, InventoryError, Product};

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum CatalogRepositoryError {
    #[error("product was not found")]
    ProductNotFound,
    #[error("inventory was not found")]
    InventoryNotFound,
    #[error("inventory already exists")]
    InventoryAlreadyExists,
    #[error("inventory operation failed: {0}")]
    Inventory(#[from] InventoryError),
}

#[async_trait]
pub trait CatalogRepository: Clone + Send + Sync + 'static {
    async fn save_product(&self, product: Product) -> Result<(), CatalogRepositoryError>;

    async fn find_product(
        &self,
        product_id: ProductId,
    ) -> Result<Option<Product>, CatalogRepositoryError>;

    async fn save_inventory(&self, inventory: Inventory) -> Result<(), CatalogRepositoryError>;

    async fn find_inventory(
        &self,
        variant_id: ProductVariantId,
    ) -> Result<Option<Inventory>, CatalogRepositoryError>;

    async fn reserve_stock(
        &self,
        variant_id: ProductVariantId,
        quantity: u64,
        reservation_id: StockReservationId,
    ) -> Result<(), CatalogRepositoryError>;

    async fn release_stock(
        &self,
        variant_id: ProductVariantId,
        reservation_id: StockReservationId,
    ) -> Result<(), CatalogRepositoryError>;
}
