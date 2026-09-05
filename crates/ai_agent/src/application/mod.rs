mod add_to_cart;
mod authorization;
mod auto_purchase;
mod search_products;

pub use add_to_cart::AddToCartInput;
pub use authorization::AiAuthorizationService;
pub use auto_purchase::AutoPurchaseInput;
pub use search_products::{AiProductSearchResult, SearchProductsInput};

use market_bot_cart::{CartRepository, CartService};
use market_bot_order::{OrderRepository, OrderService};

use crate::AiError;
use crate::domain::{AiAction, AiActionRecord, AiActionResult, AiActionType, Authorization};
use crate::ports::{AiRepository, CatalogFactsReader};
use market_bot_shared::OrderId;
use uuid::Uuid;

pub struct AiAgentService<R, CR, OR, F> {
    pub(crate) repository: R,
    pub(crate) cart: CartService<CR>,
    pub(crate) orders: OrderService<OR>,
    pub(crate) facts: F,
}

impl<R, CR, OR, F> Clone for AiAgentService<R, CR, OR, F>
where
    R: Clone,
    CR: Clone,
    OR: Clone,
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            repository: self.repository.clone(),
            cart: self.cart.clone(),
            orders: self.orders.clone(),
            facts: self.facts.clone(),
        }
    }
}

impl<R, CR, OR, F> AiAgentService<R, CR, OR, F>
where
    R: AiRepository,
    CR: CartRepository,
    OR: OrderRepository,
    F: CatalogFactsReader,
{
    pub fn new(repository: R, cart: CartService<CR>, orders: OrderService<OR>, facts: F) -> Self {
        Self {
            repository,
            cart,
            orders,
            facts,
        }
    }

    pub fn cart_service(&self) -> &CartService<CR> {
        &self.cart
    }

    pub fn order_service(&self) -> &OrderService<OR> {
        &self.orders
    }

    pub fn authorizations(&self) -> AiAuthorizationService<R> {
        AiAuthorizationService::new(self.repository.clone())
    }

    pub(crate) async fn persist_action(&self, action: AiAction) -> Result<(), AiError> {
        self.repository
            .save_action(action)
            .await
            .map_err(AiError::Repository)
    }

    pub(crate) fn action_from_error(
        authorization: Option<&Authorization>,
        action_type: AiActionType,
        input_summary: String,
        request_id: Uuid,
        error: &AiError,
    ) -> AiAction {
        AiAction::new(AiActionRecord {
            authorization_id: authorization.map(Authorization::id),
            subject_user_id: authorization.map(Authorization::subject_user_id),
            action_type,
            input_summary,
            result: AiActionResult::from_error(error),
            request_id,
            order_id: None,
            error_code: Some(error.audit_code().to_owned()),
        })
    }

    pub(crate) fn action_success(
        authorization: &Authorization,
        action_type: AiActionType,
        input_summary: String,
        request_id: Uuid,
        order_id: Option<OrderId>,
    ) -> AiAction {
        AiAction::new(AiActionRecord {
            authorization_id: Some(authorization.id()),
            subject_user_id: Some(authorization.subject_user_id()),
            action_type,
            input_summary,
            result: AiActionResult::Succeeded,
            request_id,
            order_id,
            error_code: None,
        })
    }
}

#[cfg(test)]
mod application_tests;
