use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use market_bot_shared::{DigitalAssetId, OrderId, ProductId};
use tokio::sync::Mutex;

use crate::domain::digital_delivery::{DigitalAsset, DigitalAssetType};
use crate::errors::FulfillmentError;
use crate::ports::digital_asset_store::DigitalAssetStore;

#[derive(Default)]
struct AssetState {
    assets: HashMap<DigitalAssetId, DigitalAsset>,
}

#[derive(Clone, Default)]
pub struct InMemoryDigitalAssetStore {
    state: Arc<Mutex<AssetState>>,
}

impl InMemoryDigitalAssetStore {
    pub async fn save(&self, asset: DigitalAsset) -> Result<(), FulfillmentError> {
        self.state.lock().await.assets.insert(asset.id(), asset);
        Ok(())
    }

    pub async fn unassigned_count(&self, product_id: ProductId) -> Result<usize, FulfillmentError> {
        Ok(self
            .state
            .lock()
            .await
            .assets
            .values()
            .filter(|asset| {
                asset.product_id() == product_id
                    && asset.asset_type().is_one_time_credential()
                    && asset.assigned_order_id().is_none()
            })
            .count())
    }

    pub async fn find_assigned_to_order(
        &self,
        order_id: OrderId,
    ) -> Result<Option<DigitalAsset>, FulfillmentError> {
        Ok(self
            .state
            .lock()
            .await
            .assets
            .values()
            .find(|asset| asset.assigned_order_id() == Some(order_id))
            .cloned())
    }

    pub async fn find_file_for_product(
        &self,
        product_id: ProductId,
    ) -> Result<Option<DigitalAsset>, FulfillmentError> {
        Ok(self
            .state
            .lock()
            .await
            .assets
            .values()
            .find(|asset| {
                asset.product_id() == product_id && asset.asset_type() == DigitalAssetType::File
            })
            .cloned())
    }

    pub async fn allocate_for_order(
        &self,
        product_id: ProductId,
        order_id: OrderId,
    ) -> Result<DigitalAsset, FulfillmentError> {
        let mut state = self.state.lock().await;

        // Invariant: retries and concurrent fulfills of the same order must
        // reuse the already-assigned credential instead of taking a second
        // card from the pool.
        if let Some(existing) = state.assets.values().find(|asset| {
            asset.assigned_order_id() == Some(order_id)
                && asset.asset_type().is_one_time_credential()
        }) {
            return Ok(existing.clone());
        }

        let allocated_id = state
            .assets
            .values()
            .find(|asset| {
                asset.product_id() == product_id
                    && asset.asset_type().is_one_time_credential()
                    && asset.assigned_order_id().is_none()
            })
            .map(DigitalAsset::id)
            .ok_or(FulfillmentError::NoAvailableAsset)?;

        let allocated = state
            .assets
            .get_mut(&allocated_id)
            .ok_or(FulfillmentError::NoAvailableAsset)?;
        allocated.assign_to(order_id);
        Ok(allocated.clone())
    }
}

#[async_trait]
impl DigitalAssetStore for InMemoryDigitalAssetStore {
    async fn save(&self, asset: DigitalAsset) -> Result<(), FulfillmentError> {
        InMemoryDigitalAssetStore::save(self, asset).await
    }

    async fn unassigned_count(&self, product_id: ProductId) -> Result<usize, FulfillmentError> {
        InMemoryDigitalAssetStore::unassigned_count(self, product_id).await
    }

    async fn find_assigned_to_order(
        &self,
        order_id: OrderId,
    ) -> Result<Option<DigitalAsset>, FulfillmentError> {
        InMemoryDigitalAssetStore::find_assigned_to_order(self, order_id).await
    }

    async fn find_file_for_product(
        &self,
        product_id: ProductId,
    ) -> Result<Option<DigitalAsset>, FulfillmentError> {
        InMemoryDigitalAssetStore::find_file_for_product(self, product_id).await
    }

    async fn allocate_for_order(
        &self,
        product_id: ProductId,
        order_id: OrderId,
    ) -> Result<DigitalAsset, FulfillmentError> {
        InMemoryDigitalAssetStore::allocate_for_order(self, product_id, order_id).await
    }
}
