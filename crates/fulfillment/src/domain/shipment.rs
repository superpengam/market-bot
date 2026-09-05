use chrono::{DateTime, Utc};
use market_bot_shared::{OrderId, ShipmentId};
use serde::{Deserialize, Serialize};

use crate::errors::FulfillmentError;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ShipmentStatus {
    LabelCreated,
    InTransit,
    Delivered,
    Exception,
    Returned,
}

impl ShipmentStatus {
    /// Successful physical delivery. Exception and Returned are never success.
    pub const fn is_successful_delivery(self) -> bool {
        matches!(self, Self::Delivered)
    }

    fn rank(self) -> u8 {
        match self {
            Self::LabelCreated => 0,
            Self::InTransit => 1,
            Self::Delivered | Self::Exception => 2,
            Self::Returned => 3,
        }
    }

    /// Invariant: logistics callbacks are at-least-once and may arrive late or
    /// out of order. A later `InTransit` must never undo `Delivered`.
    pub fn advance_to(self, incoming: Self) -> Self {
        if self == incoming {
            return self;
        }
        if self.is_regression(incoming) {
            return self;
        }
        incoming
    }

    fn is_regression(self, incoming: Self) -> bool {
        match (self, incoming) {
            (Self::Delivered, Self::LabelCreated | Self::InTransit | Self::Exception) => true,
            (Self::Returned, _) => true,
            (Self::Exception, Self::LabelCreated | Self::InTransit) => true,
            (Self::InTransit, Self::LabelCreated) => true,
            _ => self.rank() > incoming.rank(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Shipment {
    id: ShipmentId,
    order_id: OrderId,
    tracking_number: String,
    carrier: String,
    status: ShipmentStatus,
    updated_at: DateTime<Utc>,
}

impl Shipment {
    pub fn new(
        order_id: OrderId,
        tracking_number: impl Into<String>,
        carrier: impl Into<String>,
    ) -> Result<Self, FulfillmentError> {
        let tracking_number = tracking_number.into();
        let carrier = carrier.into();
        if tracking_number.trim().is_empty() {
            return Err(FulfillmentError::BlankTrackingNumber);
        }
        if carrier.trim().is_empty() {
            return Err(FulfillmentError::BlankCarrier);
        }

        Ok(Self {
            id: ShipmentId::new(),
            order_id,
            tracking_number,
            carrier,
            status: ShipmentStatus::LabelCreated,
            updated_at: Utc::now(),
        })
    }

    pub const fn id(&self) -> ShipmentId {
        self.id
    }

    pub const fn order_id(&self) -> OrderId {
        self.order_id
    }

    pub fn tracking_number(&self) -> &str {
        &self.tracking_number
    }

    pub fn carrier(&self) -> &str {
        &self.carrier
    }

    pub const fn status(&self) -> ShipmentStatus {
        self.status
    }

    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    pub const fn is_successful_delivery(&self) -> bool {
        self.status.is_successful_delivery()
    }

    /// Applies a mapped logistics status without moving backwards.
    ///
    /// Why: carriers retry webhooks. Duplicate `Delivered` is a no-op; a stale
    /// `InTransit` after `Delivered` must keep the package delivered.
    pub fn apply_status(&mut self, incoming: ShipmentStatus) {
        self.status = self.status.advance_to(incoming);
        self.updated_at = Utc::now();
    }
}
