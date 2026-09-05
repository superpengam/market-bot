use async_trait::async_trait;

use crate::domain::{ScanAsset, ScanResult};

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ScannerError {
    #[error("content scanner is unavailable")]
    Unavailable,
    #[error("scan asset is invalid")]
    InvalidAsset,
}

#[async_trait]
pub trait ContentScanner: Clone + Send + Sync + 'static {
    async fn scan(&self, asset: ScanAsset) -> Result<ScanResult, ScannerError>;
}
