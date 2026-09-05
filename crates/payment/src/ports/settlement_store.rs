use async_trait::async_trait;
use market_bot_shared::{OrderId, SettlementId};

use crate::domain::settlement::{Settlement, SettlementError};

#[async_trait]
pub trait SettlementStore: Clone + Send + Sync + 'static {
    async fn save(&self, settlement: Settlement) -> Result<(), SettlementError>;

    async fn find_by_order(&self, order_id: OrderId)
    -> Result<Option<Settlement>, SettlementError>;

    async fn find_by_id(
        &self,
        settlement_id: SettlementId,
    ) -> Result<Option<Settlement>, SettlementError>;
}
