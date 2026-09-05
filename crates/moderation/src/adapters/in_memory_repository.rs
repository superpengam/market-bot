use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use market_bot_seller::SellerStatus;
use market_bot_shared::{ProductId, UserId};
use serde_json::Value;
use tokio::sync::RwLock;

use crate::{
    domain::{ModerationCase, ModerationCaseId, ReportId, ScanResult},
    ports::{ListingFacts, ModerationRepository, ModerationRepositoryError, StoredReport},
};

#[derive(Default)]
struct ModerationState {
    cases: HashMap<ModerationCaseId, ModerationCase>,
    product_cases: HashMap<ProductId, ModerationCaseId>,
    report_cases: HashMap<ReportId, ModerationCaseId>,
    reports: HashMap<ReportId, StoredReport>,
    scans: HashMap<ProductId, ScanResult>,
    seller_statuses: HashMap<ProductId, SellerStatus>,
    listing_facts: HashMap<ProductId, ListingFacts>,
    idempotency: HashMap<(UserId, String, String), Value>,
}

#[derive(Clone, Default)]
pub struct InMemoryModerationRepository {
    state: Arc<RwLock<ModerationState>>,
}

#[async_trait]
impl ModerationRepository for InMemoryModerationRepository {
    async fn save_case(&self, case: ModerationCase) -> Result<(), ModerationRepositoryError> {
        let mut state = self.state.write().await;
        if case.case_kind() == crate::domain::ModerationCaseKind::ProductReview {
            state
                .product_cases
                .insert(ProductId::from_uuid(case.subject_id()), case.id());
        }
        if let Some(report_id) = case.report_id() {
            state.report_cases.insert(report_id, case.id());
        }
        state.cases.insert(case.id(), case);
        Ok(())
    }

    async fn find_case(
        &self,
        case_id: ModerationCaseId,
    ) -> Result<Option<ModerationCase>, ModerationRepositoryError> {
        Ok(self.state.read().await.cases.get(&case_id).cloned())
    }

    async fn find_product_review_case(
        &self,
        product_id: ProductId,
    ) -> Result<Option<ModerationCase>, ModerationRepositoryError> {
        let state = self.state.read().await;
        Ok(state
            .product_cases
            .get(&product_id)
            .and_then(|case_id| state.cases.get(case_id))
            .cloned())
    }

    async fn find_report_case(
        &self,
        report_id: ReportId,
    ) -> Result<Option<ModerationCase>, ModerationRepositoryError> {
        let state = self.state.read().await;
        Ok(state
            .report_cases
            .get(&report_id)
            .and_then(|case_id| state.cases.get(case_id))
            .cloned())
    }

    async fn save_report(&self, report: StoredReport) -> Result<(), ModerationRepositoryError> {
        self.state
            .write()
            .await
            .reports
            .insert(report.report_id, report);
        Ok(())
    }

    async fn find_report(
        &self,
        report_id: ReportId,
    ) -> Result<Option<StoredReport>, ModerationRepositoryError> {
        Ok(self.state.read().await.reports.get(&report_id).cloned())
    }

    async fn save_scan(&self, scan: ScanResult) -> Result<(), ModerationRepositoryError> {
        self.state
            .write()
            .await
            .scans
            .insert(scan.product_id(), scan);
        Ok(())
    }

    async fn latest_scan(
        &self,
        product_id: ProductId,
    ) -> Result<Option<ScanResult>, ModerationRepositoryError> {
        Ok(self.state.read().await.scans.get(&product_id).cloned())
    }

    async fn save_publish_context(
        &self,
        product_id: ProductId,
        seller_status: SellerStatus,
        listing_facts: ListingFacts,
    ) -> Result<(), ModerationRepositoryError> {
        let mut state = self.state.write().await;
        state.seller_statuses.insert(product_id, seller_status);
        state.listing_facts.insert(product_id, listing_facts);
        Ok(())
    }

    async fn seller_status(
        &self,
        product_id: ProductId,
    ) -> Result<Option<SellerStatus>, ModerationRepositoryError> {
        Ok(self
            .state
            .read()
            .await
            .seller_statuses
            .get(&product_id)
            .copied())
    }

    async fn listing_facts(
        &self,
        product_id: ProductId,
    ) -> Result<Option<ListingFacts>, ModerationRepositoryError> {
        Ok(self
            .state
            .read()
            .await
            .listing_facts
            .get(&product_id)
            .cloned())
    }

    async fn find_idempotent_result(
        &self,
        actor_id: UserId,
        scope: &str,
        key: &str,
    ) -> Result<Option<Value>, ModerationRepositoryError> {
        Ok(self
            .state
            .read()
            .await
            .idempotency
            .get(&(actor_id, scope.to_owned(), key.to_owned()))
            .cloned())
    }

    async fn save_idempotent_result(
        &self,
        actor_id: UserId,
        scope: &str,
        key: &str,
        result: Value,
    ) -> Result<(), ModerationRepositoryError> {
        self.state
            .write()
            .await
            .idempotency
            .insert((actor_id, scope.to_owned(), key.to_owned()), result);
        Ok(())
    }
}
