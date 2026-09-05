use axum::{Json, extract::Query, extract::State, http::StatusCode};
use market_bot_search::SearchProductsQuery;
use market_bot_shared::{CurrencyCode, Page};
use serde::Deserialize;

use crate::app::AppState;

#[derive(Debug, Deserialize)]
pub struct ProductSearchParams {
    pub q: Option<String>,
    pub category_id: Option<String>,
    pub currency: Option<String>,
    pub min_price_minor: Option<i64>,
    pub max_price_minor: Option<i64>,
    pub cursor: Option<String>,
}

pub async fn search_products(
    State(state): State<AppState>,
    Query(params): Query<ProductSearchParams>,
) -> Result<Json<Page<market_bot_search::ProductSearchResult>>, StatusCode> {
    let currency = params
        .currency
        .map(CurrencyCode::try_from)
        .transpose()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    let page = state
        .search
        .search(SearchProductsQuery {
            query: params.q,
            category_id: params.category_id,
            currency,
            min_price_minor: params.min_price_minor,
            max_price_minor: params.max_price_minor,
            cursor: params.cursor,
        })
        .await
        .map_err(|error| match error {
            market_bot_search::SearchError::InvalidPriceRange
            | market_bot_search::SearchError::InvalidCursor => StatusCode::BAD_REQUEST,
            market_bot_search::SearchError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    Ok(Json(page))
}
