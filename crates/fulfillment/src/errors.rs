use market_bot_order::{OrderError, OrderRepositoryError};

use crate::{
    domain::shipment::ShipmentStatus,
    ports::{logistics_provider::LogisticsError, object_storage::StorageError},
};

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum FulfillmentError {
    #[error("order was not found")]
    OrderNotFound,
    #[error("order is not paid and has no existing delivery receipt")]
    OrderNotPaid,
    #[error("order does not contain a digital item")]
    NotDigitalOrder,
    #[error("order does not contain a physical item")]
    NotPhysicalOrder,
    #[error("no unassigned digital credential is available for this product")]
    NoAvailableAsset,
    #[error("digital asset reference is not a valid v1 payload")]
    InvalidEncryptedReference,
    #[error("tracking number cannot be blank")]
    BlankTrackingNumber,
    #[error("carrier cannot be blank")]
    BlankCarrier,
    #[error("shipment was not found")]
    ShipmentNotFound,
    #[error("shipment status cannot move from {from:?} to {to:?}")]
    InvalidShipmentTransition {
        from: ShipmentStatus,
        to: ShipmentStatus,
    },
    #[error("order operation failed: {0}")]
    Order(#[from] OrderError),
    #[error("order repository failed: {0}")]
    OrderRepository(#[from] OrderRepositoryError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Logistics(#[from] LogisticsError),
}
