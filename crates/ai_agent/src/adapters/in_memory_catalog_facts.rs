use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use market_bot_catalog::{CatalogRepository, InMemoryCatalogRepository};
use market_bot_shared::{ProductId, ProductVariantId};
use tokio::sync::RwLock;

use crate::ports::{
    CatalogFactsError, CatalogFactsReader, CatalogListingExtras, CatalogPurchaseFacts,
};

#[derive(Clone)]
pub struct InMemoryCatalogFacts {
    catalog: InMemoryCatalogRepository,
    extras: Arc<RwLock<HashMap<(ProductId, ProductVariantId), CatalogListingExtras>>>,
}

impl InMemoryCatalogFacts {
    pub fn new(catalog: InMemoryCatalogRepository) -> Self {
        Self {
            catalog,
            extras: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn catalog(&self) -> &InMemoryCatalogRepository {
        &self.catalog
    }

    pub async fn register_extras(
        &self,
        extras: CatalogListingExtras,
    ) -> Result<(), CatalogFactsError> {
        self.extras
            .write()
            .await
            .insert((extras.product_id, extras.variant_id), extras);
        Ok(())
    }

    async fn facts_from_keys(
        &self,
        product_id: ProductId,
        variant_id: ProductVariantId,
    ) -> Result<Option<CatalogPurchaseFacts>, CatalogFactsError> {
        let extras = self
            .extras
            .read()
            .await
            .get(&(product_id, variant_id))
            .cloned();
        let Some(extras) = extras else {
            return Ok(None);
        };
        let product = self
            .catalog
            .find_product(product_id)
            .await
            .map_err(|_| CatalogFactsError::OperationFailed)?;
        let Some(product) = product else {
            return Ok(None);
        };
        let available_stock = self
            .catalog
            .find_inventory(variant_id)
            .await
            .map_err(|_| CatalogFactsError::OperationFailed)?
            .map(|inventory| inventory.available_stock())
            .unwrap_or(0);

        // Why: checkout facts come from catalog price and stock, never from search.
        Ok(Some(CatalogPurchaseFacts {
            product_id,
            variant_id,
            seller_id: product.seller_id(),
            title: product.title().to_owned(),
            category_id: extras.category_id,
            seller_score: extras.seller_score,
            unit_price: product.price().clone(),
            shipping_fee: extras.shipping_fee,
            tax: extras.tax,
            available_stock,
            fulfillment_type: product.product_type(),
            is_published: product.can_be_added_to_cart(),
        }))
    }
}

#[async_trait]
impl CatalogFactsReader for InMemoryCatalogFacts {
    async fn load_purchase_facts(
        &self,
        product_id: ProductId,
        variant_id: ProductVariantId,
    ) -> Result<Option<CatalogPurchaseFacts>, CatalogFactsError> {
        self.facts_from_keys(product_id, variant_id).await
    }

    async fn search_listings(
        &self,
        query: Option<&str>,
        category_id: Option<&str>,
    ) -> Result<Vec<CatalogPurchaseFacts>, CatalogFactsError> {
        let keys = self.extras.read().await.keys().copied().collect::<Vec<_>>();
        let mut listings = Vec::new();
        for (product_id, variant_id) in keys {
            if let Some(facts) = self.facts_from_keys(product_id, variant_id).await? {
                listings.push(facts);
            }
        }

        let query = query
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_ascii_lowercase);
        listings.retain(|facts| {
            facts.is_published
                && facts.available_stock > 0
                && category_id.is_none_or(|category| facts.category_id == category)
                && query.as_ref().is_none_or(|term| {
                    facts.title.to_ascii_lowercase().contains(term)
                        || facts.category_id.to_ascii_lowercase().contains(term)
                })
        });
        listings.sort_by_key(|facts| facts.product_id);
        Ok(listings)
    }
}
