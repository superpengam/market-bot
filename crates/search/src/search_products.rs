use market_bot_shared::{CurrencyCode, Page, ProductId, ProductVariantId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{SearchRepository, SearchRepositoryError};

pub const DEFAULT_PAGE_SIZE: usize = 50;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProductSearchDocument {
    pub product_id: ProductId,
    pub variant_ids: Vec<ProductVariantId>,
    pub title: String,
    pub searchable_text: String,
    pub category_ids: Vec<String>,
    pub attributes: BTreeMap<String, String>,
    pub price_minor: i64,
    pub currency: CurrencyCode,
    pub available_stock: u64,
    pub is_published: bool,
    pub fulfillment_type: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SearchProductsQuery {
    pub query: Option<String>,
    pub category_id: Option<String>,
    pub currency: Option<CurrencyCode>,
    pub min_price_minor: Option<i64>,
    pub max_price_minor: Option<i64>,
    pub cursor: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProductSearchResult {
    pub product_id: ProductId,
    pub variant_ids: Vec<ProductVariantId>,
    pub title: String,
    pub attributes: BTreeMap<String, String>,
    pub price_minor: i64,
    pub currency: CurrencyCode,
    pub available_stock: u64,
    pub fulfillment_type: String,
}

impl From<ProductSearchDocument> for ProductSearchResult {
    fn from(document: ProductSearchDocument) -> Self {
        Self {
            product_id: document.product_id,
            variant_ids: document.variant_ids,
            title: document.title,
            attributes: document.attributes,
            price_minor: document.price_minor,
            currency: document.currency,
            available_stock: document.available_stock,
            fulfillment_type: document.fulfillment_type,
        }
    }
}

#[derive(Clone)]
pub struct SearchService<R> {
    repository: R,
}

impl<R> SearchService<R>
where
    R: SearchRepository,
{
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn search(
        &self,
        query: SearchProductsQuery,
    ) -> Result<Page<ProductSearchResult>, SearchError> {
        if let (Some(minimum), Some(maximum)) = (query.min_price_minor, query.max_price_minor)
            && (minimum < 0 || maximum < minimum)
        {
            return Err(SearchError::InvalidPriceRange);
        }

        let offset = parse_cursor(query.cursor.as_deref())?;
        let normalized_query = query
            .query
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase);
        let documents = self
            .repository
            .list_documents()
            .await
            .map_err(SearchError::Repository)?;

        let mut matches = documents
            .into_iter()
            .filter(|document| document.is_published && document.available_stock > 0)
            .filter(|document| {
                normalized_query.as_ref().is_none_or(|term| {
                    document.title.to_ascii_lowercase().contains(term)
                        || document.searchable_text.to_ascii_lowercase().contains(term)
                })
            })
            .filter(|document| {
                query
                    .category_id
                    .as_ref()
                    .is_none_or(|category| document.category_ids.contains(category))
            })
            .filter(|document| {
                query
                    .currency
                    .as_ref()
                    .is_none_or(|currency| document.currency == *currency)
            })
            .filter(|document| {
                query
                    .min_price_minor
                    .is_none_or(|minimum| document.price_minor >= minimum)
            })
            .filter(|document| {
                query
                    .max_price_minor
                    .is_none_or(|maximum| document.price_minor <= maximum)
            })
            .collect::<Vec<_>>();

        matches.sort_by_key(|document| document.product_id);
        let page_start = offset.min(matches.len());
        let page_end = (page_start + DEFAULT_PAGE_SIZE).min(matches.len());
        let next_cursor = (page_end < matches.len()).then(|| page_end.to_string());
        let items = matches
            .drain(page_start..page_end)
            .map(ProductSearchResult::from)
            .collect();

        Ok(Page::new(items, next_cursor))
    }
}

fn parse_cursor(cursor: Option<&str>) -> Result<usize, SearchError> {
    cursor
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| SearchError::InvalidCursor)
        })
        .transpose()
        .map(|offset| offset.unwrap_or_default())
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum SearchError {
    #[error("search price range is invalid")]
    InvalidPriceRange,
    #[error("search cursor is invalid")]
    InvalidCursor,
    #[error("search repository failed: {0}")]
    Repository(#[source] SearchRepositoryError),
}
