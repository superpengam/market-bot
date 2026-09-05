use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use market_bot_order::{Order, OrderRepository, OrderStatus};
use market_bot_shared::{FulfillmentType, OrderId, ProductId};
use tokio::sync::Mutex;

use crate::domain::digital_delivery::DeliveryReceipt;
use crate::errors::FulfillmentError;
use crate::ports::digital_asset_store::DigitalAssetStore;
use crate::ports::object_storage::ObjectStorage;

#[derive(Clone)]
pub struct DigitalDeliveryService<O, A, S> {
    orders: O,
    assets: A,
    storage: S,
    ttl: Duration,
    receipts: Arc<Mutex<HashMap<OrderId, DeliveryReceipt>>>,
}

impl<O, A, S> DigitalDeliveryService<O, A, S>
where
    O: OrderRepository,
    A: DigitalAssetStore,
    S: ObjectStorage,
{
    pub fn new(orders: O, assets: A, storage: S, ttl: Duration) -> Self {
        Self {
            orders,
            assets,
            storage,
            ttl,
            receipts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Delivers a paid digital order exactly once.
    ///
    /// Invariant: a retry returns the original receipt (`fulfillment_id` and
    /// `revealed_secret`). File orders mint an expiring opaque URL; card and
    /// redeem orders atomically consume one unassigned credential.
    pub async fn fulfill(&self, order_id: OrderId) -> Result<DeliveryReceipt, FulfillmentError> {
        if let Some(existing) = self.receipts.lock().await.get(&order_id).cloned() {
            return Ok(existing);
        }

        let order = self
            .orders
            .find(order_id)
            .await?
            .ok_or(FulfillmentError::OrderNotFound)?;
        if order.status() != OrderStatus::Paid {
            return Err(FulfillmentError::OrderNotPaid);
        }

        let product_id = digital_product_id(&order)?;
        let receipt = self.deliver(order_id, product_id).await?;

        let mut receipts = self.receipts.lock().await;
        // Why: two concurrent fulfills of the same order can both pass the
        // empty-receipt check. Keep the first persisted receipt so the buyer
        // always sees one fulfillment_id.
        Ok(receipts.entry(order_id).or_insert(receipt).clone())
    }

    async fn deliver(
        &self,
        order_id: OrderId,
        product_id: ProductId,
    ) -> Result<DeliveryReceipt, FulfillmentError> {
        if let Some(assigned) = self.assets.find_assigned_to_order(order_id).await? {
            return Ok(DeliveryReceipt::credential(
                order_id,
                assigned.reveal_secret()?,
            ));
        }

        if let Some(file) = self.assets.find_file_for_product(product_id).await? {
            let expires_at = Utc::now() + self.ttl;
            let download_url = self
                .storage
                .create_download_url(file.id(), expires_at)
                .await?;
            return Ok(DeliveryReceipt::file(order_id, download_url, expires_at));
        }

        let allocated = self.assets.allocate_for_order(product_id, order_id).await?;
        Ok(DeliveryReceipt::credential(
            order_id,
            allocated.reveal_secret()?,
        ))
    }
}

fn digital_product_id(order: &Order) -> Result<ProductId, FulfillmentError> {
    order
        .items()
        .iter()
        .find(|item| item.fulfillment_type() == FulfillmentType::Digital)
        .map(|item| item.product_id())
        .ok_or(FulfillmentError::NotDigitalOrder)
}

#[async_trait]
pub trait DigitalFulfillment: Send + Sync {
    async fn fulfill(&self, order_id: OrderId) -> Result<DeliveryReceipt, FulfillmentError>;
}

#[async_trait]
impl<O, A, S> DigitalFulfillment for DigitalDeliveryService<O, A, S>
where
    O: OrderRepository,
    A: DigitalAssetStore,
    S: ObjectStorage,
{
    async fn fulfill(&self, order_id: OrderId) -> Result<DeliveryReceipt, FulfillmentError> {
        DigitalDeliveryService::fulfill(self, order_id).await
    }
}
