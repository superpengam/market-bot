//! Digital delivery and physical shipment domain module.

pub mod adapters;
pub mod application;
pub mod domain;
pub mod errors;
pub mod ports;

pub use adapters::in_memory_digital_asset_store::InMemoryDigitalAssetStore;
pub use adapters::s3_storage::S3ObjectStorage;
pub use adapters::sandbox_logistics::SandboxLogisticsProvider;
pub use application::fulfill_digital_order::{DigitalDeliveryService, DigitalFulfillment};
pub use application::update_shipment::{
    CreateShipmentCommand, ShipmentService, ShipmentSynchronizer,
};
pub use domain::digital_delivery::{DeliveryReceipt, DigitalAsset, DigitalAssetType};
pub use domain::shipment::{Shipment, ShipmentStatus};
pub use errors::FulfillmentError;
pub use ports::digital_asset_store::DigitalAssetStore;
pub use ports::logistics_provider::{LogisticsError, LogisticsProvider, map_logistics_status};
pub use ports::object_storage::{DownloadUrl, ObjectStorage, StorageError};

#[cfg(test)]
mod digital_delivery_tests;
#[cfg(test)]
mod schema_tests;
#[cfg(test)]
mod shipment_tests;
