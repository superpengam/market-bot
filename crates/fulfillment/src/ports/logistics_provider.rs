use crate::domain::shipment::ShipmentStatus;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum LogisticsError {
    #[error("tracking number was not found")]
    TrackingNotFound,
    #[error("logistics status {0} is not recognized")]
    UnrecognizedStatus(String),
    #[error("logistics provider is temporarily unavailable")]
    TemporarilyUnavailable,
}

/// Maps a carrier-specific status string onto the platform shipment machine.
pub fn map_logistics_status(raw: &str) -> Result<ShipmentStatus, LogisticsError> {
    let normalized = raw.trim().to_ascii_lowercase().replace(['-', ' '], "_");
    match normalized.as_str() {
        "label_created" | "labelcreated" => Ok(ShipmentStatus::LabelCreated),
        "in_transit" | "intransit" => Ok(ShipmentStatus::InTransit),
        "delivered" => Ok(ShipmentStatus::Delivered),
        "exception" => Ok(ShipmentStatus::Exception),
        "returned" => Ok(ShipmentStatus::Returned),
        other => Err(LogisticsError::UnrecognizedStatus(other.to_owned())),
    }
}

#[async_trait::async_trait]
pub trait LogisticsProvider: Clone + Send + Sync + 'static {
    async fn get_tracking_status(
        &self,
        tracking_number: &str,
    ) -> Result<ShipmentStatus, LogisticsError>;
}
