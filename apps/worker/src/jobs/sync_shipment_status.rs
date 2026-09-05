use market_bot_fulfillment::{FulfillmentError, Shipment, ShipmentSynchronizer};
use market_bot_shared::ShipmentId;

#[derive(Clone)]
pub struct SyncShipmentStatusJob<U> {
    update: U,
}

impl<U> SyncShipmentStatusJob<U> {
    pub fn new(update: U) -> Self {
        Self { update }
    }
}

impl<U> SyncShipmentStatusJob<U>
where
    U: ShipmentSynchronizer,
{
    /// Pulls the latest mapped carrier status and applies it without regressing.
    pub async fn run(&self, shipment_id: ShipmentId) -> Result<Shipment, FulfillmentError> {
        self.update.sync_status(shipment_id).await
    }
}
