use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use market_bot_order::{Order, OrderRepository, OrderStatus};
use market_bot_shared::{FulfillmentType, OrderId, ShipmentId};
use tokio::sync::Mutex;

use crate::domain::shipment::{Shipment, ShipmentStatus};
use crate::errors::FulfillmentError;
use crate::ports::logistics_provider::LogisticsProvider;

#[derive(Clone, Debug)]
pub struct CreateShipmentCommand {
    pub order_id: OrderId,
    pub tracking_number: String,
    pub carrier: String,
}

#[derive(Default)]
struct ShipmentState {
    by_id: HashMap<ShipmentId, Shipment>,
    by_order: HashMap<OrderId, ShipmentId>,
    by_tracking: HashMap<String, ShipmentId>,
}

#[derive(Clone)]
pub struct ShipmentService<O, L> {
    orders: O,
    logistics: L,
    state: Arc<Mutex<ShipmentState>>,
}

impl<O, L> ShipmentService<O, L>
where
    O: OrderRepository,
    L: LogisticsProvider,
{
    pub fn new(orders: O, logistics: L) -> Self {
        Self {
            orders,
            logistics,
            state: Arc::new(Mutex::new(ShipmentState::default())),
        }
    }

    /// Creates a shipment for a paid physical order and moves the order to
    /// `Shipped`.
    ///
    /// Invariant: a retry with the same order returns the original shipment
    /// instead of opening a second tracking record.
    pub async fn create_shipment(
        &self,
        command: CreateShipmentCommand,
    ) -> Result<Shipment, FulfillmentError> {
        let mut order = self
            .orders
            .find(command.order_id)
            .await?
            .ok_or(FulfillmentError::OrderNotFound)?;
        if !is_physical(&order) {
            return Err(FulfillmentError::NotPhysicalOrder);
        }

        if let Some(existing) = self.find_by_order(command.order_id).await? {
            self.ensure_order_shipped(&mut order).await?;
            return Ok(existing);
        }

        self.ensure_order_shipped(&mut order).await?;
        let shipment = Shipment::new(command.order_id, command.tracking_number, command.carrier)?;

        let mut state = self.state.lock().await;
        state.by_order.insert(shipment.order_id(), shipment.id());
        state
            .by_tracking
            .insert(shipment.tracking_number().to_owned(), shipment.id());
        state.by_id.insert(shipment.id(), shipment.clone());
        Ok(shipment)
    }

    pub async fn get_shipment(
        &self,
        shipment_id: ShipmentId,
    ) -> Result<Option<Shipment>, FulfillmentError> {
        Ok(self.state.lock().await.by_id.get(&shipment_id).cloned())
    }

    pub async fn apply_status(
        &self,
        shipment_id: ShipmentId,
        incoming: ShipmentStatus,
    ) -> Result<Shipment, FulfillmentError> {
        let mut state = self.state.lock().await;
        let shipment = state
            .by_id
            .get_mut(&shipment_id)
            .ok_or(FulfillmentError::ShipmentNotFound)?;
        shipment.apply_status(incoming);
        Ok(shipment.clone())
    }

    /// Pulls the latest mapped carrier status and applies it idempotently.
    pub async fn sync_status(&self, shipment_id: ShipmentId) -> Result<Shipment, FulfillmentError> {
        let current = self
            .get_shipment(shipment_id)
            .await?
            .ok_or(FulfillmentError::ShipmentNotFound)?;
        let incoming = self
            .logistics
            .get_tracking_status(current.tracking_number())
            .await?;
        self.apply_status(shipment_id, incoming).await
    }

    pub async fn apply_callback(
        &self,
        tracking_number: &str,
        incoming: ShipmentStatus,
    ) -> Result<Shipment, FulfillmentError> {
        let shipment_id = {
            let state = self.state.lock().await;
            *state
                .by_tracking
                .get(tracking_number)
                .ok_or(FulfillmentError::ShipmentNotFound)?
        };
        self.apply_status(shipment_id, incoming).await
    }

    async fn find_by_order(&self, order_id: OrderId) -> Result<Option<Shipment>, FulfillmentError> {
        let state = self.state.lock().await;
        Ok(state
            .by_order
            .get(&order_id)
            .and_then(|shipment_id| state.by_id.get(shipment_id))
            .cloned())
    }

    async fn ensure_order_shipped(&self, order: &mut Order) -> Result<(), FulfillmentError> {
        match order.status() {
            OrderStatus::Paid => {
                order.transition_to(OrderStatus::FulfillmentProcessing)?;
                order.transition_to(OrderStatus::Shipped)?;
            }
            OrderStatus::FulfillmentProcessing => {
                order.transition_to(OrderStatus::Shipped)?;
            }
            OrderStatus::Shipped | OrderStatus::Delivered => {}
            _ => return Err(FulfillmentError::OrderNotPaid),
        }

        self.orders.update(order.clone()).await?;
        Ok(())
    }
}

fn is_physical(order: &Order) -> bool {
    order
        .items()
        .iter()
        .any(|item| item.fulfillment_type() == FulfillmentType::PhysicalStandard)
}

#[async_trait]
pub trait ShipmentSynchronizer: Send + Sync {
    async fn sync_status(&self, shipment_id: ShipmentId) -> Result<Shipment, FulfillmentError>;
}

#[async_trait]
impl<O, L> ShipmentSynchronizer for ShipmentService<O, L>
where
    O: OrderRepository,
    L: LogisticsProvider,
{
    async fn sync_status(&self, shipment_id: ShipmentId) -> Result<Shipment, FulfillmentError> {
        ShipmentService::sync_status(self, shipment_id).await
    }
}
