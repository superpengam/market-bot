use market_bot_catalog::Product;
use market_bot_shared::CartId;

use crate::{AddCartItem, Cart, CartError, CartItem, CartRepository, CartRepositoryError};

#[derive(Clone)]
pub struct CartService<R> {
    repository: R,
}

impl<R> CartService<R>
where
    R: CartRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create_cart(
        &self,
        owner_id: market_bot_shared::UserId,
    ) -> Result<Cart, CartServiceError> {
        let cart = Cart::new(owner_id);
        self.repository
            .save(cart.clone())
            .await
            .map_err(CartServiceError::Repository)?;
        Ok(cart)
    }

    pub async fn get_cart(&self, cart_id: CartId) -> Result<Option<Cart>, CartServiceError> {
        self.repository
            .find(cart_id)
            .await
            .map_err(CartServiceError::Repository)
    }

    pub async fn add_item(
        &self,
        cart_id: CartId,
        item: AddCartItem,
    ) -> Result<CartItem, CartServiceError> {
        let mut cart = self.load_cart(cart_id).await?;
        let cart_item = cart.add_item(item).map_err(CartServiceError::Cart)?;
        self.repository
            .save(cart)
            .await
            .map_err(CartServiceError::Repository)?;
        Ok(cart_item)
    }

    pub async fn add_purchasable_item(
        &self,
        cart_id: CartId,
        item: AddCartItem,
        product: &Product,
    ) -> Result<CartItem, CartServiceError> {
        if !product.can_be_added_to_cart() {
            return Err(CartServiceError::ProductNotPurchasable);
        }

        self.add_item(cart_id, item).await
    }

    pub async fn update_quantity(
        &self,
        cart_id: CartId,
        item_id: market_bot_shared::CartItemId,
        quantity: u64,
    ) -> Result<(), CartServiceError> {
        let mut cart = self.load_cart(cart_id).await?;
        cart.update_quantity(item_id, quantity)
            .map_err(CartServiceError::Cart)?;
        self.repository
            .save(cart)
            .await
            .map_err(CartServiceError::Repository)
    }

    pub async fn remove_item(
        &self,
        cart_id: CartId,
        item_id: market_bot_shared::CartItemId,
    ) -> Result<bool, CartServiceError> {
        let mut cart = self.load_cart(cart_id).await?;
        let removed = cart.remove_item(item_id);
        self.repository
            .save(cart)
            .await
            .map_err(CartServiceError::Repository)?;
        Ok(removed)
    }

    async fn load_cart(&self, cart_id: CartId) -> Result<Cart, CartServiceError> {
        self.repository
            .find(cart_id)
            .await
            .map_err(CartServiceError::Repository)?
            .ok_or(CartServiceError::CartNotFound)
    }
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum CartServiceError {
    #[error("cart was not found")]
    CartNotFound,
    #[error("product cannot be added to the cart")]
    ProductNotPurchasable,
    #[error("cart operation failed: {0}")]
    Cart(#[source] CartError),
    #[error("cart repository failed: {0}")]
    Repository(#[source] CartRepositoryError),
}
