use market_bot_cart::{AddCartItem, CartItem, CartItemSource, CartServiceError};
use market_bot_shared::{CartId, ProductId, ProductVariantId};
use uuid::Uuid;

use super::AiAgentService;
use crate::AiError;
use crate::domain::{AiActionType, AiScope};
use crate::ports::{AiRepository, CatalogFactsReader};
use market_bot_cart::CartRepository;
use market_bot_order::OrderRepository;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AddToCartInput {
    pub authorization_id: market_bot_shared::AiAuthorizationId,
    pub cart_id: CartId,
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub quantity: u64,
    pub request_id: Uuid,
}

impl<R, CR, OR, F> AiAgentService<R, CR, OR, F>
where
    R: AiRepository,
    CR: CartRepository,
    OR: OrderRepository,
    F: CatalogFactsReader,
{
    pub async fn add_to_cart(&self, input: AddToCartInput) -> Result<CartItem, AiError> {
        let summary = format!(
            "add_to_cart cart={} product={} variant={} quantity={}",
            input.cart_id, input.product_id, input.variant_id, input.quantity
        );
        match self.add_to_cart_inner(&input).await {
            Ok((authorization, item)) => {
                self.persist_action(Self::action_success(
                    &authorization,
                    AiActionType::AddToCart,
                    summary,
                    input.request_id,
                    None,
                ))
                .await?;
                Ok(item)
            }
            Err(error) => {
                let authorization = self
                    .repository
                    .find_authorization(input.authorization_id)
                    .await
                    .ok()
                    .flatten();
                self.persist_action(Self::action_from_error(
                    authorization.as_ref(),
                    AiActionType::AddToCart,
                    summary,
                    input.request_id,
                    &error,
                ))
                .await?;
                Err(error)
            }
        }
    }

    async fn add_to_cart_inner(
        &self,
        input: &AddToCartInput,
    ) -> Result<(crate::domain::Authorization, CartItem), AiError> {
        if input.quantity == 0 {
            return Err(AiError::InvalidQuantity);
        }

        let authorization = self
            .authorizations()
            .require_scope(input.authorization_id, AiScope::CartWrite)
            .await?;
        let facts = self
            .facts
            .load_purchase_facts(input.product_id, input.variant_id)
            .await
            .map_err(AiError::CatalogFacts)?
            .ok_or(AiError::ProductNotFound)?;

        // Safety: unpublished or suspended listings cannot enter a cart through AI.
        if !facts.is_published {
            return Err(AiError::ProductNotPurchasable);
        }
        if facts.available_stock < input.quantity {
            return Err(AiError::ProductOutOfStock);
        }

        let cart = self
            .cart
            .get_cart(input.cart_id)
            .await
            .map_err(AiError::Cart)?
            .ok_or(AiError::CartNotFound)?;
        if cart.owner_id() != authorization.subject_user_id() {
            return Err(AiError::CartOwnerMismatch);
        }

        let item = self
            .cart
            .add_item(
                input.cart_id,
                AddCartItem {
                    product_id: facts.product_id,
                    variant_id: facts.variant_id,
                    title: facts.title,
                    unit_price: facts.unit_price,
                    quantity: input.quantity,
                    source: CartItemSource::Ai,
                    fulfillment_type: facts.fulfillment_type,
                },
            )
            .await
            .map_err(|error| match error {
                CartServiceError::CartNotFound => AiError::CartNotFound,
                other => AiError::Cart(other),
            })?;
        Ok((authorization, item))
    }
}
