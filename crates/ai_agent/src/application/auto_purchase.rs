use chrono::{DateTime, Datelike, TimeZone, Utc};
use market_bot_order::{CreateOrderCommand, Order, OrderItem};
use market_bot_shared::{ProductId, ProductVariantId};
use uuid::Uuid;

use super::AiAgentService;
use crate::AiError;
use crate::domain::{AiActionType, AiScope, Authorization, PurchaseEvaluation};
use crate::ports::{AiRepository, CatalogFactsReader, CatalogPurchaseFacts};
use market_bot_cart::CartRepository;
use market_bot_order::OrderRepository;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoPurchaseInput {
    pub authorization_id: market_bot_shared::AiAuthorizationId,
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub quantity: u64,
    pub quoted_unit_price_minor: i64,
    pub quoted_shipping_minor: i64,
    pub idempotency_key: String,
    pub request_id: Uuid,
    pub now: DateTime<Utc>,
}

impl<R, CR, OR, F> AiAgentService<R, CR, OR, F>
where
    R: AiRepository,
    CR: CartRepository,
    OR: OrderRepository,
    F: CatalogFactsReader,
{
    pub async fn auto_purchase(&self, input: AutoPurchaseInput) -> Result<Order, AiError> {
        let summary = format!(
            "auto_purchase product={} variant={} quantity={} quoted_unit_price_minor={} quoted_shipping_minor={}",
            input.product_id,
            input.variant_id,
            input.quantity,
            input.quoted_unit_price_minor,
            input.quoted_shipping_minor
        );
        match self.auto_purchase_inner(&input).await {
            Ok((authorization, order)) => {
                self.persist_action(Self::action_success(
                    &authorization,
                    AiActionType::AutoPurchase,
                    summary,
                    input.request_id,
                    Some(order.id()),
                ))
                .await?;
                Ok(order)
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
                    AiActionType::AutoPurchase,
                    summary,
                    input.request_id,
                    &error,
                ))
                .await?;
                Err(error)
            }
        }
    }

    async fn auto_purchase_inner(
        &self,
        input: &AutoPurchaseInput,
    ) -> Result<(Authorization, Order), AiError> {
        if input.quantity == 0 {
            return Err(AiError::InvalidQuantity);
        }
        if input.idempotency_key.trim().is_empty() {
            return Err(AiError::BlankIdempotencyKey);
        }

        // Safety: AutoPurchase is a distinct grant. OrderCreate never satisfies this check.
        let authorization = self
            .authorizations()
            .require_scope(input.authorization_id, AiScope::AutoPurchase)
            .await?;

        // Why: auto-pay stays opt-in even when the client already holds the scope.
        if !self
            .authorizations()
            .is_auto_purchase_enabled(authorization.subject_user_id())
            .await?
        {
            return Err(AiError::AutoPurchaseDisabled);
        }

        let policy = self
            .repository
            .find_policy(authorization.subject_user_id())
            .await
            .map_err(AiError::Repository)?
            .ok_or(AiError::PolicyNotFound)?;

        // Why: search quotes are untrusted; catalog is the purchase fact source.
        let facts = self
            .facts
            .load_purchase_facts(input.product_id, input.variant_id)
            .await
            .map_err(AiError::CatalogFacts)?
            .ok_or(AiError::ProductNotFound)?;
        if !facts.is_published {
            return Err(AiError::ProductNotPurchasable);
        }
        if facts.available_stock < input.quantity {
            return Err(AiError::ProductOutOfStock);
        }

        let line_total = facts
            .unit_price
            .clone()
            .checked_mul(input.quantity)
            .map_err(|_| AiError::InvalidPolicyAmount)?;
        let order_total = line_total
            .clone()
            .checked_add(facts.shipping_fee.clone())
            .and_then(|value| value.checked_add(facts.tax.clone()))
            .map_err(|_| AiError::InvalidPolicyAmount)?;

        let daily_spent_minor = self
            .repository
            .spend_between(
                authorization.subject_user_id(),
                start_of_utc_day(input.now),
                input.now,
            )
            .await
            .map_err(AiError::Repository)?;
        let monthly_spent_minor = self
            .repository
            .spend_between(
                authorization.subject_user_id(),
                start_of_utc_month(input.now),
                input.now,
            )
            .await
            .map_err(AiError::Repository)?;

        let evaluation = PurchaseEvaluation {
            category_id: facts.category_id.clone(),
            seller_score: facts.seller_score,
            order_total_minor: order_total.minor(),
            shipping_minor: facts.shipping_fee.minor(),
            daily_spent_minor,
            monthly_spent_minor,
            quoted_unit_price_minor: input.quoted_unit_price_minor,
            catalog_unit_price_minor: facts.unit_price.minor(),
            quoted_shipping_minor: input.quoted_shipping_minor,
            catalog_shipping_minor: facts.shipping_fee.minor(),
        };

        // Safety: only PurchasePolicy::evaluate may choose Allowed/Confirm/Blocked.
        policy.evaluate(&evaluation).into_result()?;

        let order = self
            .create_order_from_facts(&authorization, input, &facts)
            .await?;
        self.repository
            .record_spend(
                authorization.subject_user_id(),
                order.total().minor(),
                input.now,
            )
            .await
            .map_err(AiError::Repository)?;
        Ok((authorization, order))
    }

    async fn create_order_from_facts(
        &self,
        authorization: &Authorization,
        input: &AutoPurchaseInput,
        facts: &CatalogPurchaseFacts,
    ) -> Result<Order, AiError> {
        let item = OrderItem::new(
            facts.product_id,
            facts.variant_id,
            facts.seller_id,
            facts.title.clone(),
            facts.unit_price.clone(),
            input.quantity,
            facts.fulfillment_type,
        )
        .map_err(|error| AiError::Order(market_bot_order::OrderServiceError::Order(error)))?;

        self.orders
            .create_order(CreateOrderCommand {
                buyer_id: authorization.subject_user_id(),
                items: vec![item],
                shipping_fee: facts.shipping_fee.clone(),
                tax: facts.tax.clone(),
                idempotency_key: input.idempotency_key.clone(),
            })
            .await
            .map_err(AiError::Order)
    }
}

fn start_of_utc_day(now: DateTime<Utc>) -> DateTime<Utc> {
    now.date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|naive| Utc.from_utc_datetime(&naive))
        .unwrap_or(now)
}

fn start_of_utc_month(now: DateTime<Utc>) -> DateTime<Utc> {
    chrono::NaiveDate::from_ymd_opt(now.year(), now.month(), 1)
        .and_then(|date| date.and_hms_opt(0, 0, 0))
        .map(|naive| Utc.from_utc_datetime(&naive))
        .unwrap_or(now)
}
