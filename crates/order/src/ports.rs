use async_trait::async_trait;
use market_bot_shared::{OrderId, UserId};

use crate::Order;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum OrderRepositoryError {
    #[error("order storage operation failed")]
    OperationFailed,
}

#[async_trait]
pub trait OrderRepository: Clone + Send + Sync + 'static {
    async fn get_or_create_by_idempotency(
        &self,
        buyer_id: UserId,
        idempotency_key: &str,
        order: Order,
    ) -> Result<Order, OrderRepositoryError>;

    async fn find(&self, order_id: OrderId) -> Result<Option<Order>, OrderRepositoryError>;

    async fn update(&self, order: Order) -> Result<(), OrderRepositoryError>;
}
