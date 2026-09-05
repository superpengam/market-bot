use async_trait::async_trait;
use market_bot_shared::CartId;

use crate::Cart;

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum CartRepositoryError {
    #[error("cart storage operation failed")]
    OperationFailed,
}

#[async_trait]
pub trait CartRepository: Clone + Send + Sync + 'static {
    async fn save(&self, cart: Cart) -> Result<(), CartRepositoryError>;
    async fn find(&self, cart_id: CartId) -> Result<Option<Cart>, CartRepositoryError>;
}
