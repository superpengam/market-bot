use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use market_bot_shared::{OrderId, SettlementId};
use tokio::sync::Mutex;

use crate::{
    domain::settlement::{Settlement, SettlementError},
    ports::settlement_store::SettlementStore,
};

#[derive(Default)]
struct SettlementState {
    by_id: HashMap<SettlementId, Settlement>,
    by_order: HashMap<OrderId, SettlementId>,
}

#[derive(Clone, Default)]
pub struct InMemorySettlementStore {
    state: Arc<Mutex<SettlementState>>,
}

#[async_trait]
impl SettlementStore for InMemorySettlementStore {
    async fn save(&self, settlement: Settlement) -> Result<(), SettlementError> {
        let mut state = self.state.lock().await;
        if let Some(existing_id) = state.by_order.get(&settlement.order_id())
            && *existing_id != settlement.id()
        {
            return Err(SettlementError::AlreadyExists);
        }

        state
            .by_order
            .insert(settlement.order_id(), settlement.id());
        state.by_id.insert(settlement.id(), settlement);
        Ok(())
    }

    async fn find_by_order(
        &self,
        order_id: OrderId,
    ) -> Result<Option<Settlement>, SettlementError> {
        let state = self.state.lock().await;
        Ok(state
            .by_order
            .get(&order_id)
            .and_then(|id| state.by_id.get(id))
            .cloned())
    }

    async fn find_by_id(
        &self,
        settlement_id: SettlementId,
    ) -> Result<Option<Settlement>, SettlementError> {
        Ok(self.state.lock().await.by_id.get(&settlement_id).cloned())
    }
}
