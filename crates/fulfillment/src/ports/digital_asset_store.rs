use market_bot_shared::{OrderId, ProductId};

use crate::domain::digital_delivery::DigitalAsset;
use crate::errors::FulfillmentError;

#[async_trait::async_trait]
pub trait DigitalAssetStore: Clone + Send + Sync + 'static {
    async fn save(&self, asset: DigitalAsset) -> Result<(), FulfillmentError>;

    async fn unassigned_count(&self, product_id: ProductId) -> Result<usize, FulfillmentError>;

    async fn find_assigned_to_order(
        &self,
        order_id: OrderId,
    ) -> Result<Option<DigitalAsset>, FulfillmentError>;

    async fn find_file_for_product(
        &self,
        product_id: ProductId,
    ) -> Result<Option<DigitalAsset>, FulfillmentError>;

    /// Picks one unassigned card/redeem for the product and marks it owned by
    /// `order_id`.
    ///
    /// Invariant: the pick and the assignment happen under one lock so two
    /// concurrent fulfills cannot receive the same secret. A retry for an
    /// order that already owns a credential must return that credential.
    async fn allocate_for_order(
        &self,
        product_id: ProductId,
        order_id: OrderId,
    ) -> Result<DigitalAsset, FulfillmentError>;
}
