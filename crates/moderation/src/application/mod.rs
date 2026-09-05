mod create_report;
mod review_product;

pub use create_report::CreateReportInput;
pub use review_product::{PublishCheckFailure, PublishReadiness};

use market_bot_catalog::CatalogRepository;
use market_bot_search::SearchRepository;
use market_bot_shared::{OutboxStore, UserId};
use serde_json::Value;

use crate::ModerationError;

use crate::ports::{ContentScanner, ModerationRepository};

pub struct ModerationService<R, C, S, O, Sc> {
    pub(crate) repository: R,
    pub(crate) catalog: C,
    pub(crate) search: S,
    pub(crate) outbox: O,
    pub(crate) scanner: Sc,
}

impl<R, C, S, O, Sc> Clone for ModerationService<R, C, S, O, Sc>
where
    R: Clone,
    C: Clone,
    S: Clone,
    O: Clone,
    Sc: Clone,
{
    fn clone(&self) -> Self {
        Self {
            repository: self.repository.clone(),
            catalog: self.catalog.clone(),
            search: self.search.clone(),
            outbox: self.outbox.clone(),
            scanner: self.scanner.clone(),
        }
    }
}

impl<R, C, S, O, Sc> ModerationService<R, C, S, O, Sc>
where
    R: ModerationRepository,
    C: CatalogRepository,
    S: SearchRepository,
    O: OutboxStore,
    Sc: ContentScanner,
{
    pub fn new(repository: R, catalog: C, search: S, outbox: O, scanner: Sc) -> Self {
        Self {
            repository,
            catalog,
            search,
            outbox,
            scanner,
        }
    }

    pub async fn idempotent_result(
        &self,
        actor_id: UserId,
        scope: &str,
        key: &str,
    ) -> Result<Option<Value>, ModerationError> {
        self.repository
            .find_idempotent_result(actor_id, scope, key)
            .await
            .map_err(ModerationError::Repository)
    }

    pub async fn store_idempotent_result(
        &self,
        actor_id: UserId,
        scope: &str,
        key: &str,
        result: Value,
    ) -> Result<(), ModerationError> {
        self.repository
            .save_idempotent_result(actor_id, scope, key, result)
            .await
            .map_err(ModerationError::Repository)
    }
}
