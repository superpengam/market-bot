use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use market_bot_shared::CartId;
use tokio::sync::RwLock;

use crate::{Cart, CartRepository, CartRepositoryError};

#[derive(Clone, Default)]
pub struct InMemoryCartRepository {
    carts: Arc<RwLock<HashMap<CartId, Cart>>>,
}

#[async_trait]
impl CartRepository for InMemoryCartRepository {
    async fn save(&self, cart: Cart) -> Result<(), CartRepositoryError> {
        self.carts.write().await.insert(cart.id(), cart);
        Ok(())
    }

    async fn find(&self, cart_id: CartId) -> Result<Option<Cart>, CartRepositoryError> {
        Ok(self.carts.read().await.get(&cart_id).cloned())
    }
}
