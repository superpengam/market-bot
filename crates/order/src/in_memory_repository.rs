use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use market_bot_shared::{OrderId, UserId};
use tokio::sync::Mutex;

use crate::{Order, OrderRepository, OrderRepositoryError};

#[derive(Default)]
struct RepositoryState {
    orders: HashMap<OrderId, Order>,
    idempotency: HashMap<(UserId, String), OrderId>,
}

#[derive(Clone, Default)]
pub struct InMemoryOrderRepository {
    state: Arc<Mutex<RepositoryState>>,
}

#[async_trait]
impl OrderRepository for InMemoryOrderRepository {
    async fn get_or_create_by_idempotency(
        &self,
        buyer_id: UserId,
        idempotency_key: &str,
        order: Order,
    ) -> Result<Order, OrderRepositoryError> {
        let mut state = self.state.lock().await;
        let key = (buyer_id, idempotency_key.to_owned());
        if let Some(order_id) = state.idempotency.get(&key) {
            return state
                .orders
                .get(order_id)
                .cloned()
                .ok_or(OrderRepositoryError::OperationFailed);
        }

        let order_id = order.id();
        state.idempotency.insert(key, order_id);
        state.orders.insert(order_id, order.clone());
        Ok(order)
    }

    async fn find(&self, order_id: OrderId) -> Result<Option<Order>, OrderRepositoryError> {
        Ok(self.state.lock().await.orders.get(&order_id).cloned())
    }

    async fn update(&self, order: Order) -> Result<(), OrderRepositoryError> {
        let mut state = self.state.lock().await;
        if !state.orders.contains_key(&order.id()) {
            return Err(OrderRepositoryError::OperationFailed);
        }

        state.orders.insert(order.id(), order);
        Ok(())
    }
}
