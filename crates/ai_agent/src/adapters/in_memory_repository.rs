use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use market_bot_shared::{AiAuthorizationId, UserId};
use tokio::sync::RwLock;

use crate::domain::{AiAction, Authorization, PurchasePolicy};
use crate::ports::{AiRepository, AiRepositoryError};

#[derive(Default)]
struct AiState {
    authorizations: HashMap<AiAuthorizationId, Authorization>,
    policies: HashMap<UserId, PurchasePolicy>,
    auto_purchase_enabled: HashMap<UserId, bool>,
    actions: Vec<AiAction>,
    spend: Vec<(UserId, i64, DateTime<Utc>)>,
}

#[derive(Clone, Default)]
pub struct InMemoryAiRepository {
    state: Arc<RwLock<AiState>>,
}

#[async_trait]
impl AiRepository for InMemoryAiRepository {
    async fn save_authorization(
        &self,
        authorization: Authorization,
    ) -> Result<(), AiRepositoryError> {
        self.state
            .write()
            .await
            .authorizations
            .insert(authorization.id(), authorization);
        Ok(())
    }

    async fn find_authorization(
        &self,
        authorization_id: AiAuthorizationId,
    ) -> Result<Option<Authorization>, AiRepositoryError> {
        Ok(self
            .state
            .read()
            .await
            .authorizations
            .get(&authorization_id)
            .cloned())
    }

    async fn save_policy(
        &self,
        user_id: UserId,
        policy: PurchasePolicy,
    ) -> Result<(), AiRepositoryError> {
        self.state.write().await.policies.insert(user_id, policy);
        Ok(())
    }

    async fn find_policy(
        &self,
        user_id: UserId,
    ) -> Result<Option<PurchasePolicy>, AiRepositoryError> {
        Ok(self.state.read().await.policies.get(&user_id).cloned())
    }

    async fn set_auto_purchase_enabled(
        &self,
        user_id: UserId,
        enabled: bool,
    ) -> Result<(), AiRepositoryError> {
        self.state
            .write()
            .await
            .auto_purchase_enabled
            .insert(user_id, enabled);
        Ok(())
    }

    async fn is_auto_purchase_enabled(&self, user_id: UserId) -> Result<bool, AiRepositoryError> {
        Ok(self
            .state
            .read()
            .await
            .auto_purchase_enabled
            .get(&user_id)
            .copied()
            .unwrap_or(false))
    }

    async fn save_action(&self, action: AiAction) -> Result<(), AiRepositoryError> {
        self.state.write().await.actions.push(action);
        Ok(())
    }

    async fn list_actions(&self, user_id: UserId) -> Result<Vec<AiAction>, AiRepositoryError> {
        Ok(self
            .state
            .read()
            .await
            .actions
            .iter()
            .filter(|action| action.subject_user_id() == Some(user_id))
            .cloned()
            .collect())
    }

    async fn record_spend(
        &self,
        user_id: UserId,
        amount_minor: i64,
        at: DateTime<Utc>,
    ) -> Result<(), AiRepositoryError> {
        self.state
            .write()
            .await
            .spend
            .push((user_id, amount_minor, at));
        Ok(())
    }

    async fn spend_between(
        &self,
        user_id: UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<i64, AiRepositoryError> {
        Ok(self
            .state
            .read()
            .await
            .spend
            .iter()
            .filter(|(owner, _, at)| *owner == user_id && *at >= from && *at <= to)
            .map(|(_, amount, _)| *amount)
            .fold(0_i64, |total, amount| total.saturating_add(amount)))
    }
}
