use chrono::{DateTime, Utc};
use market_bot_shared::{AiAuthorizationId, UserId};
use uuid::Uuid;

use crate::AiError;
use crate::domain::{
    AiAction, AiActionRecord, AiActionResult, AiActionType, AiClientId, AiScope, Authorization,
    PurchasePolicy,
};
use crate::ports::AiRepository;

#[derive(Clone)]
pub struct AiAuthorizationService<R> {
    repository: R,
}

impl<R> AiAuthorizationService<R>
where
    R: AiRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn authorize(
        &self,
        subject: UserId,
        client: impl AsRef<str>,
        scopes: impl IntoIterator<Item = AiScope>,
        expires_at: DateTime<Utc>,
    ) -> Result<Authorization, AiError> {
        let client_id = AiClientId::parse(client)?;
        let authorization = Authorization::new(subject, client_id, scopes, expires_at);
        self.repository
            .save_authorization(authorization.clone())
            .await
            .map_err(AiError::Repository)?;
        self.persist_action(AiAction::new(AiActionRecord {
            authorization_id: Some(authorization.id()),
            subject_user_id: Some(subject),
            action_type: AiActionType::Authorize,
            input_summary: format!(
                "authorize client={} scopes={}",
                authorization.client_id(),
                authorization
                    .scopes()
                    .map(|scope| scope.as_api_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            result: AiActionResult::Succeeded,
            request_id: Uuid::new_v4(),
            order_id: None,
            error_code: None,
        }))
        .await?;
        Ok(authorization)
    }

    pub async fn revoke(
        &self,
        authorization_id: AiAuthorizationId,
    ) -> Result<Authorization, AiError> {
        let mut authorization = self.load(authorization_id).await?;
        authorization.revoke(Utc::now());
        self.repository
            .save_authorization(authorization.clone())
            .await
            .map_err(AiError::Repository)?;
        self.persist_action(AiAction::new(AiActionRecord {
            authorization_id: Some(authorization.id()),
            subject_user_id: Some(authorization.subject_user_id()),
            action_type: AiActionType::Revoke,
            input_summary: format!("revoke authorization={}", authorization.id()),
            result: AiActionResult::Succeeded,
            request_id: Uuid::new_v4(),
            order_id: None,
            error_code: None,
        }))
        .await?;
        Ok(authorization)
    }

    /// Safety: expired and revoked grants are rejected before any cart or order write.
    pub async fn require_scope(
        &self,
        authorization_id: AiAuthorizationId,
        scope: AiScope,
    ) -> Result<Authorization, AiError> {
        let authorization = self.load(authorization_id).await?;
        authorization.require_scope(scope, Utc::now())?;
        Ok(authorization)
    }

    pub async fn save_policy(
        &self,
        user_id: UserId,
        policy: PurchasePolicy,
    ) -> Result<(), AiError> {
        self.repository
            .save_policy(user_id, policy)
            .await
            .map_err(AiError::Repository)
    }

    pub async fn set_auto_purchase_enabled(
        &self,
        user_id: UserId,
        enabled: bool,
    ) -> Result<(), AiError> {
        self.repository
            .set_auto_purchase_enabled(user_id, enabled)
            .await
            .map_err(AiError::Repository)
    }

    pub async fn is_auto_purchase_enabled(&self, user_id: UserId) -> Result<bool, AiError> {
        self.repository
            .is_auto_purchase_enabled(user_id)
            .await
            .map_err(AiError::Repository)
    }

    pub async fn list_actions(
        &self,
        user_id: UserId,
    ) -> Result<Vec<crate::domain::AiAction>, AiError> {
        self.repository
            .list_actions(user_id)
            .await
            .map_err(AiError::Repository)
    }

    async fn load(&self, authorization_id: AiAuthorizationId) -> Result<Authorization, AiError> {
        self.repository
            .find_authorization(authorization_id)
            .await
            .map_err(AiError::Repository)?
            .ok_or(AiError::AuthorizationNotFound)
    }

    async fn persist_action(&self, action: AiAction) -> Result<(), AiError> {
        self.repository
            .save_action(action)
            .await
            .map_err(AiError::Repository)
    }
}
