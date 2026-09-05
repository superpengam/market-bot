use market_bot_shared::ProductId;
use uuid::Uuid;

use super::AiAgentService;
use crate::AiError;
use crate::domain::{AiActionType, AiScope};
use crate::ports::{AiRepository, CatalogFactsReader};
use market_bot_cart::CartRepository;
use market_bot_order::OrderRepository;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchProductsInput {
    pub authorization_id: market_bot_shared::AiAuthorizationId,
    pub query: Option<String>,
    pub category_id: Option<String>,
    pub request_id: Uuid,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AiProductSearchResult {
    pub product_id: ProductId,
    pub variant_id: market_bot_shared::ProductVariantId,
    pub title: String,
    pub category_id: String,
    pub price_minor: i64,
    pub available_stock: u64,
}

impl<R, CR, OR, F> AiAgentService<R, CR, OR, F>
where
    R: AiRepository,
    CR: CartRepository,
    OR: OrderRepository,
    F: CatalogFactsReader,
{
    pub async fn search_products(
        &self,
        input: SearchProductsInput,
    ) -> Result<Vec<AiProductSearchResult>, AiError> {
        let summary = format!(
            "search_products query={} category={}",
            input.query.as_deref().unwrap_or(""),
            input.category_id.as_deref().unwrap_or("")
        );
        match self.search_products_inner(&input).await {
            Ok((authorization, results)) => {
                self.persist_action(Self::action_success(
                    &authorization,
                    AiActionType::SearchProducts,
                    summary,
                    input.request_id,
                    None,
                ))
                .await?;
                Ok(results)
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
                    AiActionType::SearchProducts,
                    summary,
                    input.request_id,
                    &error,
                ))
                .await?;
                Err(error)
            }
        }
    }

    async fn search_products_inner(
        &self,
        input: &SearchProductsInput,
    ) -> Result<(crate::domain::Authorization, Vec<AiProductSearchResult>), AiError> {
        let authorization = self
            .authorizations()
            .require_scope(input.authorization_id, AiScope::CatalogRead)
            .await?;
        let listings = self
            .facts
            .search_listings(input.query.as_deref(), input.category_id.as_deref())
            .await
            .map_err(AiError::CatalogFacts)?;
        let results = listings
            .into_iter()
            .map(|facts| AiProductSearchResult {
                product_id: facts.product_id,
                variant_id: facts.variant_id,
                title: facts.title,
                category_id: facts.category_id,
                price_minor: facts.unit_price.minor(),
                available_stock: facts.available_stock,
            })
            .collect();
        Ok((authorization, results))
    }
}
