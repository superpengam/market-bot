use chrono::{DateTime, Utc};
use market_bot_shared::{Money, OrderId, SellerId};

use crate::{
    domain::settlement::{Settlement, SettlementBlockReason, SettlementError, SettlementStatus},
    ports::{
        payment_provider::{PaymentProvider, ProviderError, SettlementReleaseInput},
        settlement_store::SettlementStore,
    },
};

#[derive(Clone, Debug)]
pub struct CreateSettlementInput {
    pub order_id: OrderId,
    pub seller_id: SellerId,
    pub amount: Money,
    pub auto_confirm_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct SettlementService<S, P> {
    store: S,
    provider: P,
}

impl<S, P> SettlementService<S, P>
where
    S: SettlementStore,
    P: PaymentProvider,
{
    pub fn new(store: S, provider: P) -> Self {
        Self { store, provider }
    }

    pub async fn create_pending(
        &self,
        input: CreateSettlementInput,
    ) -> Result<Settlement, SettlementError> {
        if self.store.find_by_order(input.order_id).await?.is_some() {
            return Err(SettlementError::AlreadyExists);
        }

        let settlement = Settlement::pending(
            input.order_id,
            input.seller_id,
            input.amount,
            input.auto_confirm_at,
        );
        self.store.save(settlement.clone()).await?;
        Ok(settlement)
    }

    pub async fn get_by_order(
        &self,
        order_id: OrderId,
    ) -> Result<Option<Settlement>, SettlementError> {
        self.store.find_by_order(order_id).await
    }

    pub async fn record_digital_delivered(
        &self,
        order_id: OrderId,
    ) -> Result<Settlement, SettlementError> {
        let mut settlement = self.require(order_id).await?;
        settlement.record_digital_delivered(Utc::now())?;
        self.store.save(settlement.clone()).await?;
        Ok(settlement)
    }

    pub async fn record_block(
        &self,
        order_id: OrderId,
        reason: SettlementBlockReason,
    ) -> Result<Settlement, SettlementError> {
        let mut settlement = self.require(order_id).await?;
        settlement.record_block(reason)?;
        self.store.save(settlement.clone()).await?;
        Ok(settlement)
    }

    pub async fn mark_eligible(&self, order_id: OrderId) -> Result<Settlement, SettlementError> {
        let mut settlement = self.require(order_id).await?;
        settlement.mark_eligible(Utc::now())?;
        self.store.save(settlement.clone()).await?;
        Ok(settlement)
    }

    /// Safety: a blocked settlement never reaches `PaymentProvider::release_settlement`.
    pub async fn release(&self, order_id: OrderId) -> Result<Settlement, SettlementError> {
        let mut settlement = self.require(order_id).await?;
        settlement.ensure_releasable()?;

        let intent = self
            .provider
            .release_settlement(SettlementReleaseInput {
                settlement_id: settlement.id(),
                order_id: settlement.order_id(),
                seller_id: settlement.seller_id(),
                amount: settlement.amount().clone(),
            })
            .await
            .map_err(map_provider_error)?;

        settlement.mark_released(intent.provider_settlement_id())?;
        debug_assert_eq!(settlement.status(), SettlementStatus::Released);
        debug_assert!(settlement.wallet_balance().is_none());
        self.store.save(settlement.clone()).await?;
        Ok(settlement)
    }

    async fn require(&self, order_id: OrderId) -> Result<Settlement, SettlementError> {
        self.store
            .find_by_order(order_id)
            .await?
            .ok_or(SettlementError::NotFound)
    }
}

fn map_provider_error(error: ProviderError) -> SettlementError {
    match error {
        ProviderError::TemporarilyUnavailable => SettlementError::ProviderTemporarilyUnavailable,
        ProviderError::InvalidSignature | ProviderError::InvalidPayload => {
            SettlementError::ProviderFailed
        }
    }
}
