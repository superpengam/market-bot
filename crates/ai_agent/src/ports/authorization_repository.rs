use async_trait::async_trait;
use chrono::{DateTime, Utc};
use market_bot_shared::{AiAuthorizationId, UserId};

use crate::domain::{AiAction, Authorization, PurchasePolicy};

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum AiRepositoryError {
    #[error("AI storage operation failed")]
    OperationFailed,
}

#[async_trait]
pub trait AiRepository: Clone + Send + Sync + 'static {
    async fn save_authorization(
        &self,
        authorization: Authorization,
    ) -> Result<(), AiRepositoryError>;

    async fn find_authorization(
        &self,
        authorization_id: AiAuthorizationId,
    ) -> Result<Option<Authorization>, AiRepositoryError>;

    async fn save_policy(
        &self,
        user_id: UserId,
        policy: PurchasePolicy,
    ) -> Result<(), AiRepositoryError>;

    async fn find_policy(
        &self,
        user_id: UserId,
    ) -> Result<Option<PurchasePolicy>, AiRepositoryError>;

    async fn set_auto_purchase_enabled(
        &self,
        user_id: UserId,
        enabled: bool,
    ) -> Result<(), AiRepositoryError>;

    async fn is_auto_purchase_enabled(&self, user_id: UserId) -> Result<bool, AiRepositoryError>;

    async fn save_action(&self, action: AiAction) -> Result<(), AiRepositoryError>;

    async fn list_actions(&self, user_id: UserId) -> Result<Vec<AiAction>, AiRepositoryError>;

    async fn record_spend(
        &self,
        user_id: UserId,
        amount_minor: i64,
        at: DateTime<Utc>,
    ) -> Result<(), AiRepositoryError>;

    async fn spend_between(
        &self,
        user_id: UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<i64, AiRepositoryError>;
}
