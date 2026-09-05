use market_bot_catalog::{CatalogRepositoryError, ProductError};
use market_bot_search::SearchRepositoryError;
use market_bot_shared::OutboxError;

use crate::ports::{ModerationRepositoryError, ScannerError};

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ModerationError {
    #[error("moderation reason cannot be blank")]
    BlankReason,
    #[error("report details cannot be blank")]
    BlankReportDetails,
    #[error("report reason code cannot be blank")]
    BlankReasonCode,
    #[error("product was not found")]
    ProductNotFound,
    #[error("report was not found")]
    ReportNotFound,
    #[error("moderation case was not found")]
    CaseNotFound,
    #[error("product is not ready for public listing")]
    ProductNotReady,
    #[error("product cannot be added to the cart")]
    ProductNotPurchasable,
    #[error("product status transition is invalid")]
    InvalidProductStatus(#[source] ProductError),
    #[error("catalog repository failed: {0}")]
    Catalog(#[source] CatalogRepositoryError),
    #[error("search repository failed: {0}")]
    Search(#[source] SearchRepositoryError),
    #[error("outbox append failed: {0}")]
    Outbox(#[source] OutboxError),
    #[error("content scanner failed: {0}")]
    Scanner(#[source] ScannerError),
    #[error("moderation repository failed: {0}")]
    Repository(#[source] ModerationRepositoryError),
}
