use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::domain::shipment::ShipmentStatus;
use crate::ports::logistics_provider::{LogisticsError, LogisticsProvider, map_logistics_status};

#[derive(Clone, Default)]
pub struct SandboxLogisticsProvider {
    statuses: Arc<Mutex<HashMap<String, String>>>,
}

impl SandboxLogisticsProvider {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn set_status(
        &self,
        tracking_number: impl Into<String>,
        raw_status: impl Into<String>,
    ) {
        self.statuses
            .lock()
            .await
            .insert(tracking_number.into(), raw_status.into());
    }

    pub fn map_status(raw: &str) -> Result<ShipmentStatus, LogisticsError> {
        map_logistics_status(raw)
    }
}

#[async_trait]
impl LogisticsProvider for SandboxLogisticsProvider {
    async fn get_tracking_status(
        &self,
        tracking_number: &str,
    ) -> Result<ShipmentStatus, LogisticsError> {
        let statuses = self.statuses.lock().await;
        match statuses.get(tracking_number) {
            Some(raw) => map_logistics_status(raw),
            // Why: a newly created label has no carrier scan yet. Treat the
            // absence of a sandbox update as LabelCreated instead of a miss.
            None => Ok(ShipmentStatus::LabelCreated),
        }
    }
}
