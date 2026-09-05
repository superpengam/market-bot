use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use market_bot_shared::ProductId;
use tokio::sync::RwLock;

use crate::{ProductSearchDocument, SearchRepository, SearchRepositoryError};

#[derive(Clone, Default)]
pub struct InMemorySearchRepository {
    documents: Arc<RwLock<HashMap<ProductId, ProductSearchDocument>>>,
}

impl InMemorySearchRepository {
    pub async fn insert(&self, document: ProductSearchDocument) {
        self.documents
            .write()
            .await
            .insert(document.product_id, document);
    }
}

#[async_trait]
impl SearchRepository for InMemorySearchRepository {
    async fn upsert_document(
        &self,
        document: ProductSearchDocument,
    ) -> Result<(), SearchRepositoryError> {
        self.insert(document).await;
        Ok(())
    }

    async fn remove_document(&self, product_id: ProductId) -> Result<(), SearchRepositoryError> {
        self.documents.write().await.remove(&product_id);
        Ok(())
    }

    async fn list_documents(&self) -> Result<Vec<ProductSearchDocument>, SearchRepositoryError> {
        Ok(self.documents.read().await.values().cloned().collect())
    }
}
