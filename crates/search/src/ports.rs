use async_trait::async_trait;
use market_bot_shared::ProductId;

use crate::ProductSearchDocument;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum SearchRepositoryError {
    #[error("search document operation failed")]
    OperationFailed,
}

#[async_trait]
pub trait SearchRepository: Clone + Send + Sync + 'static {
    async fn upsert_document(
        &self,
        document: ProductSearchDocument,
    ) -> Result<(), SearchRepositoryError>;

    async fn remove_document(&self, product_id: ProductId) -> Result<(), SearchRepositoryError>;

    async fn list_documents(&self) -> Result<Vec<ProductSearchDocument>, SearchRepositoryError>;
}
