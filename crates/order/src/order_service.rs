use market_bot_shared::OrderId;

use crate::{Order, OrderError, OrderItem, OrderRepository, OrderRepositoryError, OrderStatus};

#[derive(Clone, Debug)]
pub struct CreateOrderCommand {
    pub buyer_id: market_bot_shared::UserId,
    pub items: Vec<OrderItem>,
    pub shipping_fee: market_bot_shared::Money,
    pub tax: market_bot_shared::Money,
    pub idempotency_key: String,
}

#[derive(Clone)]
pub struct OrderService<R> {
    repository: R,
}

impl<R> OrderService<R>
where
    R: OrderRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create_order(
        &self,
        command: CreateOrderCommand,
    ) -> Result<Order, OrderServiceError> {
        let idempotency_key = command.idempotency_key.trim();
        if idempotency_key.is_empty() {
            return Err(OrderServiceError::BlankIdempotencyKey);
        }

        let order = Order::new(
            command.buyer_id,
            command.items,
            command.shipping_fee,
            command.tax,
        )
        .map_err(OrderServiceError::Order)?;

        self.repository
            .get_or_create_by_idempotency(command.buyer_id, idempotency_key, order)
            .await
            .map_err(OrderServiceError::Repository)
    }

    pub async fn get_order(&self, order_id: OrderId) -> Result<Option<Order>, OrderServiceError> {
        self.repository
            .find(order_id)
            .await
            .map_err(OrderServiceError::Repository)
    }

    pub async fn transition_order(
        &self,
        order_id: OrderId,
        next_status: OrderStatus,
    ) -> Result<Order, OrderServiceError> {
        let mut order = self
            .repository
            .find(order_id)
            .await
            .map_err(OrderServiceError::Repository)?
            .ok_or(OrderServiceError::OrderNotFound)?;
        order
            .transition_to(next_status)
            .map_err(OrderServiceError::Order)?;
        self.repository
            .update(order.clone())
            .await
            .map_err(OrderServiceError::Repository)?;
        Ok(order)
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum OrderServiceError {
    #[error("idempotency key cannot be blank")]
    BlankIdempotencyKey,
    #[error("order was not found")]
    OrderNotFound,
    #[error("order operation failed: {0}")]
    Order(#[source] OrderError),
    #[error("order repository failed: {0}")]
    Repository(#[source] OrderRepositoryError),
}
