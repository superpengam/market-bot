use std::collections::BTreeMap;

use chrono::Utc;
use market_bot_catalog::{CatalogRepository, Product, ProductError, ProductStatus, ProductType};
use market_bot_search::{ProductSearchDocument, SearchRepository};
use market_bot_seller::SellerStatus;
use market_bot_shared::{OutboxEvent, OutboxStore, ProductId};
use serde_json::json;

use super::ModerationService;
use crate::{
    ModerationError,
    domain::{
        ModerationCase, ModerationDecision, ModerationReason, ScanAsset, ScanResult, ScanVerdict,
    },
    ports::{ContentScanner, ListingFacts, ModerationRepository, ScannerError},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublishCheckFailure {
    MissingRequiredFields,
    InvalidProductType,
    InvalidPrice,
    MissingInventory,
    FileScanFailed,
    SellerNotActive,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishReadiness {
    pub is_ready: bool,
    pub failures: Vec<PublishCheckFailure>,
}

impl<R, C, S, O, Sc> ModerationService<R, C, S, O, Sc>
where
    R: ModerationRepository,
    C: CatalogRepository,
    S: SearchRepository,
    O: OutboxStore,
    Sc: ContentScanner,
{
    pub async fn record_publish_context(
        &self,
        product_id: ProductId,
        seller_status: SellerStatus,
        listing_facts: ListingFacts,
    ) -> Result<(), ModerationError> {
        self.repository
            .save_publish_context(product_id, seller_status, listing_facts)
            .await
            .map_err(ModerationError::Repository)
    }

    pub async fn review_product(
        &self,
        product_id: ProductId,
        decision: ModerationDecision,
        reason: ModerationReason,
    ) -> Result<(), ModerationError> {
        let mut product = self.load_product(product_id).await?;
        if decision == ModerationDecision::Approved {
            let readiness = self.evaluate_publish_readiness(product_id).await?;
            if !readiness.is_ready {
                return Err(ModerationError::ProductNotReady);
            }
        }

        self.apply_visibility_decision(&mut product, decision)
            .await?;
        self.catalog
            .save_product(product.clone())
            .await
            .map_err(ModerationError::Catalog)?;
        self.append_decision_event(&product, decision, &reason)
            .await?;
        self.record_product_review_action(product_id, decision, &reason)
            .await
    }

    pub async fn product_review_case(
        &self,
        product_id: ProductId,
    ) -> Result<Option<ModerationCase>, ModerationError> {
        self.repository
            .find_product_review_case(product_id)
            .await
            .map_err(ModerationError::Repository)
    }

    pub async fn ensure_can_add_to_cart(
        &self,
        product_id: ProductId,
    ) -> Result<(), ModerationError> {
        let product = self.load_product(product_id).await?;
        if !product.can_be_added_to_cart() {
            return Err(ModerationError::ProductNotPurchasable);
        }

        Ok(())
    }

    pub async fn scan_digital_asset(
        &self,
        asset: ScanAsset,
    ) -> Result<ScanResult, ModerationError> {
        if asset.filename.trim().is_empty() {
            return Err(ModerationError::Scanner(ScannerError::InvalidAsset));
        }

        let scan = self
            .scanner
            .scan(asset)
            .await
            .map_err(ModerationError::Scanner)?;
        self.repository
            .save_scan(scan.clone())
            .await
            .map_err(ModerationError::Repository)?;
        // Why: a failed scan must never enqueue digital fulfillment; only the
        // scan verdict is published so workers can skip delivery.
        let payload = json!({
            "product_id": scan.product_id(),
            "asset_id": scan.asset_id(),
            "verdict": scan.verdict(),
            "reason_code": scan.reason_code(),
            "filename": scan.filename(),
            "occurred_at": scan.scanned_at(),
        });
        self.outbox
            .append(OutboxEvent::new(
                "file_scan.completed",
                "product",
                scan.product_id().as_uuid(),
                payload,
            ))
            .await
            .map_err(ModerationError::Outbox)?;
        Ok(scan)
    }

    pub async fn can_trigger_digital_fulfillment(
        &self,
        product_id: ProductId,
    ) -> Result<bool, ModerationError> {
        let scan = self
            .repository
            .latest_scan(product_id)
            .await
            .map_err(ModerationError::Repository)?;
        Ok(matches!(
            scan,
            Some(result) if result.verdict() == ScanVerdict::Passed
        ))
    }

    pub async fn evaluate_publish_readiness(
        &self,
        product_id: ProductId,
    ) -> Result<PublishReadiness, ModerationError> {
        let product = self.load_product(product_id).await?;
        let mut failures = Vec::new();

        if product.title().trim().is_empty() || product.description().trim().is_empty() {
            failures.push(PublishCheckFailure::MissingRequiredFields);
        }

        match product.product_type() {
            ProductType::Digital | ProductType::PhysicalStandard => {}
        }

        if product.price().minor() <= 0 {
            failures.push(PublishCheckFailure::InvalidPrice);
        }

        let seller_status = self
            .repository
            .seller_status(product_id)
            .await
            .map_err(ModerationError::Repository)?;
        if !matches!(seller_status, Some(SellerStatus::Active)) {
            failures.push(PublishCheckFailure::SellerNotActive);
        }

        let listing_facts = self
            .repository
            .listing_facts(product_id)
            .await
            .map_err(ModerationError::Repository)?;
        if listing_facts
            .as_ref()
            .is_none_or(|facts| facts.available_stock == 0)
        {
            failures.push(PublishCheckFailure::MissingInventory);
        }

        if product.product_type() == ProductType::Digital {
            let scan = self
                .repository
                .latest_scan(product_id)
                .await
                .map_err(ModerationError::Repository)?;
            if !matches!(scan, Some(result) if result.verdict() == ScanVerdict::Passed) {
                failures.push(PublishCheckFailure::FileScanFailed);
            }
        }

        Ok(PublishReadiness {
            is_ready: failures.is_empty(),
            failures,
        })
    }

    async fn load_product(&self, product_id: ProductId) -> Result<Product, ModerationError> {
        self.catalog
            .find_product(product_id)
            .await
            .map_err(ModerationError::Catalog)?
            .ok_or(ModerationError::ProductNotFound)
    }

    async fn apply_visibility_decision(
        &self,
        product: &mut Product,
        decision: ModerationDecision,
    ) -> Result<(), ModerationError> {
        match decision {
            ModerationDecision::Approved => {
                if product.status() == ProductStatus::Draft {
                    product
                        .submit_for_review()
                        .map_err(ModerationError::InvalidProductStatus)?;
                }
                if product.status() == ProductStatus::Suspended {
                    product
                        .return_to_review()
                        .map_err(ModerationError::InvalidProductStatus)?;
                }
                if product.status() == ProductStatus::PendingReview {
                    product
                        .publish()
                        .map_err(ModerationError::InvalidProductStatus)?;
                }
                if product.status() != ProductStatus::Published {
                    return Err(ModerationError::InvalidProductStatus(
                        ProductError::InvalidStatusTransition,
                    ));
                }
                self.index_product(product).await
            }
            ModerationDecision::Suspended => {
                if product.status() == ProductStatus::Draft {
                    product
                        .submit_for_review()
                        .map_err(ModerationError::InvalidProductStatus)?;
                }
                if matches!(
                    product.status(),
                    ProductStatus::PendingReview | ProductStatus::Published
                ) {
                    product
                        .suspend()
                        .map_err(ModerationError::InvalidProductStatus)?;
                }
                self.hide_product(product.id()).await
            }
            ModerationDecision::Rejected | ModerationDecision::NeedsReview => {
                if product.status() == ProductStatus::Draft
                    && decision == ModerationDecision::NeedsReview
                {
                    product
                        .submit_for_review()
                        .map_err(ModerationError::InvalidProductStatus)?;
                }
                if product.status() == ProductStatus::Published {
                    product
                        .suspend()
                        .map_err(ModerationError::InvalidProductStatus)?;
                }
                self.hide_product(product.id()).await
            }
        }
    }

    async fn index_product(&self, product: &Product) -> Result<(), ModerationError> {
        let facts = self
            .repository
            .listing_facts(product.id())
            .await
            .map_err(ModerationError::Repository)?
            .unwrap_or(ListingFacts {
                variant_ids: Vec::new(),
                available_stock: 0,
            });
        let fulfillment_type = match product.product_type() {
            ProductType::Digital => "digital",
            ProductType::PhysicalStandard => "physical_standard",
        };
        let document = ProductSearchDocument {
            product_id: product.id(),
            variant_ids: facts.variant_ids,
            title: product.title().to_owned(),
            searchable_text: format!("{} {}", product.title(), product.description()),
            category_ids: Vec::new(),
            attributes: BTreeMap::new(),
            price_minor: product.price().minor(),
            currency: product.price().currency().clone(),
            available_stock: facts.available_stock,
            is_published: product.is_publicly_visible(),
            fulfillment_type: fulfillment_type.to_owned(),
        };
        self.search
            .upsert_document(document)
            .await
            .map_err(ModerationError::Search)
    }

    async fn hide_product(&self, product_id: ProductId) -> Result<(), ModerationError> {
        self.search
            .remove_document(product_id)
            .await
            .map_err(ModerationError::Search)
    }

    async fn append_decision_event(
        &self,
        product: &Product,
        decision: ModerationDecision,
        reason: &ModerationReason,
    ) -> Result<(), ModerationError> {
        if decision == ModerationDecision::Approved && product.status() != ProductStatus::Published
        {
            return Ok(());
        }
        let event_type = match decision {
            ModerationDecision::Approved => "product.approved",
            ModerationDecision::Rejected => "product.rejected",
            ModerationDecision::Suspended => "product.suspended",
            ModerationDecision::NeedsReview => "product.review_requested",
        };
        let payload = json!({
            "product_id": product.id(),
            "decision": decision,
            "reason": reason.text(),
            "actor_id": reason.actor_id(),
            "occurred_at": Utc::now(),
        });
        self.outbox
            .append(OutboxEvent::new(
                event_type,
                "product",
                product.id().as_uuid(),
                payload,
            ))
            .await
            .map_err(ModerationError::Outbox)
    }

    async fn record_product_review_action(
        &self,
        product_id: ProductId,
        decision: ModerationDecision,
        reason: &ModerationReason,
    ) -> Result<(), ModerationError> {
        let mut case = self
            .repository
            .find_product_review_case(product_id)
            .await
            .map_err(ModerationError::Repository)?
            .unwrap_or_else(|| ModerationCase::open_product_review(product_id, Utc::now()));
        case.record_decision(reason, decision);
        self.repository
            .save_case(case)
            .await
            .map_err(ModerationError::Repository)
    }
}
