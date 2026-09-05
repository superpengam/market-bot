use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use market_bot_shared::DigitalAssetId;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::ports::object_storage::{DownloadUrl, ObjectStorage, StorageError};

#[derive(Default)]
struct StorageState {
    objects: HashMap<DigitalAssetId, String>,
}

#[derive(Clone)]
pub struct S3ObjectStorage {
    base_url: String,
    state: Arc<Mutex<StorageState>>,
}

impl S3ObjectStorage {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            state: Arc::new(Mutex::new(StorageState::default())),
        }
    }

    pub async fn put_object(
        &self,
        asset_id: DigitalAssetId,
        object_key: impl AsRef<str>,
    ) -> Result<(), StorageError> {
        self.state
            .lock()
            .await
            .objects
            .insert(asset_id, object_key.as_ref().to_owned());
        Ok(())
    }

    /// Mints `{base_url}/{opaque_token}` for a privately stored object.
    ///
    /// Safety: the token is a random UUID. The raw object key never appears in
    /// the URL returned to buyers.
    pub async fn create_download_url(
        &self,
        asset_id: DigitalAssetId,
        expires_at: DateTime<Utc>,
    ) -> Result<DownloadUrl, StorageError> {
        let state = self.state.lock().await;
        if !state.objects.contains_key(&asset_id) {
            return Err(StorageError::ObjectNotFound);
        }

        let token = Uuid::new_v4();
        let base = self.base_url.trim_end_matches('/');
        Ok(DownloadUrl::new(format!("{base}/{token}"), expires_at))
    }
}

#[async_trait]
impl ObjectStorage for S3ObjectStorage {
    async fn put_object(
        &self,
        asset_id: DigitalAssetId,
        object_key: &str,
    ) -> Result<(), StorageError> {
        S3ObjectStorage::put_object(self, asset_id, object_key).await
    }

    async fn create_download_url(
        &self,
        asset_id: DigitalAssetId,
        expires_at: DateTime<Utc>,
    ) -> Result<DownloadUrl, StorageError> {
        S3ObjectStorage::create_download_url(self, asset_id, expires_at).await
    }
}
