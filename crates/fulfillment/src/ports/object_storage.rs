use chrono::{DateTime, Utc};
use market_bot_shared::DigitalAssetId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DownloadUrl {
    url: String,
    expires_at: DateTime<Utc>,
}

impl DownloadUrl {
    pub(crate) fn new(url: impl Into<String>, expires_at: DateTime<Utc>) -> Self {
        Self {
            url: url.into(),
            expires_at,
        }
    }

    pub fn as_str(&self) -> &str {
        &self.url
    }

    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum StorageError {
    #[error("object was not found in private storage")]
    ObjectNotFound,
    #[error("object storage operation failed")]
    OperationFailed,
}

#[async_trait::async_trait]
pub trait ObjectStorage: Clone + Send + Sync + 'static {
    async fn put_object(
        &self,
        asset_id: DigitalAssetId,
        object_key: &str,
    ) -> Result<(), StorageError>;

    /// Mints an expiring URL that must not contain the raw object key.
    ///
    /// Safety: clients download through this token only. The private bucket
    /// path stays inside the adapter.
    async fn create_download_url(
        &self,
        asset_id: DigitalAssetId,
        expires_at: DateTime<Utc>,
    ) -> Result<DownloadUrl, StorageError>;
}
