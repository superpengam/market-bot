use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use market_bot_shared::{ProductId, ProductVariantId, StockReservationId};
use tokio::sync::RwLock;

use super::{CatalogRepository, CatalogRepositoryError, Inventory, Product};

#[derive(Clone, Default)]
pub struct InMemoryCatalogRepository {
    products: Arc<RwLock<HashMap<ProductId, Product>>>,
    inventories: Arc<RwLock<HashMap<ProductVariantId, Inventory>>>,
}

#[async_trait]
impl CatalogRepository for InMemoryCatalogRepository {
    async fn save_product(&self, product: Product) -> Result<(), CatalogRepositoryError> {
        self.products.write().await.insert(product.id(), product);
        Ok(())
    }

    async fn find_product(
        &self,
        product_id: ProductId,
    ) -> Result<Option<Product>, CatalogRepositoryError> {
        Ok(self.products.read().await.get(&product_id).cloned())
    }

    async fn save_inventory(&self, inventory: Inventory) -> Result<(), CatalogRepositoryError> {
        let mut inventories = self.inventories.write().await;
        if inventories.contains_key(&inventory.variant_id()) {
            return Err(CatalogRepositoryError::InventoryAlreadyExists);
        }

        inventories.insert(inventory.variant_id(), inventory);
        Ok(())
    }

    async fn find_inventory(
        &self,
        variant_id: ProductVariantId,
    ) -> Result<Option<Inventory>, CatalogRepositoryError> {
        Ok(self.inventories.read().await.get(&variant_id).cloned())
    }

    async fn reserve_stock(
        &self,
        variant_id: ProductVariantId,
        quantity: u64,
        reservation_id: StockReservationId,
    ) -> Result<(), CatalogRepositoryError> {
        let mut inventories = self.inventories.write().await;
        let inventory = inventories
            .get_mut(&variant_id)
            .ok_or(CatalogRepositoryError::InventoryNotFound)?;
        inventory.reserve(quantity, reservation_id)?;
        Ok(())
    }

    async fn release_stock(
        &self,
        variant_id: ProductVariantId,
        reservation_id: StockReservationId,
    ) -> Result<(), CatalogRepositoryError> {
        let mut inventories = self.inventories.write().await;
        let inventory = inventories
            .get_mut(&variant_id)
            .ok_or(CatalogRepositoryError::InventoryNotFound)?;
        inventory.release(reservation_id)?;
        Ok(())
    }
}
