use async_trait::async_trait;
use market_bot_seller::SellerStatus;
use market_bot_shared::{ProductId, ProductVariantId, UserId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::domain::{
    ModerationCase, ModerationCaseId, ModerationSubjectType, ReportId, ReportStatus, ScanResult,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ListingFacts {
    pub variant_ids: Vec<ProductVariantId>,
    pub available_stock: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StoredReport {
    pub report_id: ReportId,
    pub reporter_id: UserId,
    pub subject_type: ModerationSubjectType,
    pub subject_id: Uuid,
    pub reason_code: String,
    pub details: String,
    pub status: ReportStatus,
    pub case_id: ModerationCaseId,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum ModerationRepositoryError {
    #[error("moderation storage operation failed")]
    OperationFailed,
}

#[async_trait]
pub trait ModerationRepository: Clone + Send + Sync + 'static {
    async fn save_case(&self, case: ModerationCase) -> Result<(), ModerationRepositoryError>;

    async fn find_case(
        &self,
        case_id: ModerationCaseId,
    ) -> Result<Option<ModerationCase>, ModerationRepositoryError>;

    async fn find_product_review_case(
        &self,
        product_id: ProductId,
    ) -> Result<Option<ModerationCase>, ModerationRepositoryError>;

    async fn find_report_case(
        &self,
        report_id: ReportId,
    ) -> Result<Option<ModerationCase>, ModerationRepositoryError>;

    async fn save_report(&self, report: StoredReport) -> Result<(), ModerationRepositoryError>;

    async fn find_report(
        &self,
        report_id: ReportId,
    ) -> Result<Option<StoredReport>, ModerationRepositoryError>;

    async fn save_scan(&self, scan: ScanResult) -> Result<(), ModerationRepositoryError>;

    async fn latest_scan(
        &self,
        product_id: ProductId,
    ) -> Result<Option<ScanResult>, ModerationRepositoryError>;

    async fn save_publish_context(
        &self,
        product_id: ProductId,
        seller_status: SellerStatus,
        listing_facts: ListingFacts,
    ) -> Result<(), ModerationRepositoryError>;

    async fn seller_status(
        &self,
        product_id: ProductId,
    ) -> Result<Option<SellerStatus>, ModerationRepositoryError>;

    async fn listing_facts(
        &self,
        product_id: ProductId,
    ) -> Result<Option<ListingFacts>, ModerationRepositoryError>;

    async fn find_idempotent_result(
        &self,
        actor_id: UserId,
        scope: &str,
        key: &str,
    ) -> Result<Option<Value>, ModerationRepositoryError>;

    async fn save_idempotent_result(
        &self,
        actor_id: UserId,
        scope: &str,
        key: &str,
        result: Value,
    ) -> Result<(), ModerationRepositoryError>;
}
