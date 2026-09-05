//! AI authorization, purchase policy, and scoped auto-purchase.

mod adapters;
mod application;
mod domain;
mod errors;
mod ports;

pub use adapters::{InMemoryAiRepository, InMemoryCatalogFacts};
pub use application::{
    AddToCartInput, AiAgentService, AiAuthorizationService, AiProductSearchResult,
    AutoPurchaseInput, SearchProductsInput,
};
pub use domain::{
    AiAction, AiActionRecord, AiActionResult, AiActionType, AiClientId, AiScope, Authorization,
    PolicyDecision, PolicyReason, PurchaseEvaluation, PurchasePolicy, sanitize_audit_text,
};
pub use errors::AiError;
pub use market_bot_cart::{Cart, CartItem, CartItemSource, CartService, InMemoryCartRepository};
pub use market_bot_order::{InMemoryOrderRepository, Order, OrderService};
pub use ports::{
    AiRepository, AiRepositoryError, CatalogFactsError, CatalogFactsReader, CatalogListingExtras,
    CatalogPurchaseFacts,
};

use market_bot_catalog::{
    CatalogRepository, CatalogService, CreateProductCommand, InMemoryCatalogRepository, ProductType,
};
use market_bot_shared::{CurrencyCode, Money, ProductId, ProductVariantId, SellerId};

/// Fully wired in-memory stack for tests and local composition.
#[derive(Clone)]
pub struct InMemoryAiStack {
    pub repository: InMemoryAiRepository,
    pub catalog_repo: InMemoryCatalogRepository,
    pub catalog: CatalogService<InMemoryCatalogRepository>,
    pub facts: InMemoryCatalogFacts,
    pub authorizations: AiAuthorizationService<InMemoryAiRepository>,
    pub agent: AiAgentService<
        InMemoryAiRepository,
        InMemoryCartRepository,
        InMemoryOrderRepository,
        InMemoryCatalogFacts,
    >,
}

impl Default for InMemoryAiStack {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PublishedListing {
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub title: String,
    pub price_minor: i64,
}

impl InMemoryAiStack {
    pub fn new() -> Self {
        let repository = InMemoryAiRepository::default();
        let catalog_repo = InMemoryCatalogRepository::default();
        let facts = InMemoryCatalogFacts::new(catalog_repo.clone());
        let cart = CartService::new(InMemoryCartRepository::default());
        let orders = OrderService::new(InMemoryOrderRepository::default());
        Self {
            authorizations: AiAuthorizationService::new(repository.clone()),
            agent: AiAgentService::new(repository.clone(), cart, orders, facts.clone()),
            catalog: CatalogService::new(catalog_repo.clone()),
            catalog_repo,
            facts,
            repository,
        }
    }

    pub async fn publish_listing(
        &self,
        title: &str,
        price_minor: i64,
        shipping_minor: i64,
        stock: u64,
        seller_score: i32,
    ) -> Result<PublishedListing, AiError> {
        self.publish_listing_in_category(
            title,
            price_minor,
            shipping_minor,
            stock,
            seller_score,
            "electronics",
        )
        .await
    }

    pub async fn publish_listing_in_category(
        &self,
        title: &str,
        price_minor: i64,
        shipping_minor: i64,
        stock: u64,
        seller_score: i32,
        category_id: &str,
    ) -> Result<PublishedListing, AiError> {
        let mut product = self
            .catalog
            .create_product(CreateProductCommand {
                seller_id: SellerId::new(),
                title: title.to_owned(),
                description: format!("{title} description"),
                product_type: ProductType::PhysicalStandard,
                price_minor,
                currency: "USD".to_owned(),
            })
            .await
            .map_err(|_| AiError::ProductNotPurchasable)?;
        product
            .submit_for_review()
            .map_err(|_| AiError::ProductNotPurchasable)?;
        product
            .publish()
            .map_err(|_| AiError::ProductNotPurchasable)?;
        self.catalog_repo
            .save_product(product.clone())
            .await
            .map_err(AiError::Catalog)?;
        let variant_id = ProductVariantId::new();
        self.catalog
            .initialize_inventory(variant_id, stock)
            .await
            .map_err(|_| AiError::ProductOutOfStock)?;
        let currency = CurrencyCode::try_from("USD").expect("USD is valid");
        self.facts
            .register_extras(CatalogListingExtras {
                product_id: product.id(),
                variant_id,
                category_id: category_id.to_owned(),
                seller_score,
                shipping_fee: Money::new(shipping_minor, currency.clone())
                    .map_err(|_| AiError::InvalidPolicyAmount)?,
                tax: Money::new(0, currency).map_err(|_| AiError::InvalidPolicyAmount)?,
            })
            .await
            .map_err(AiError::CatalogFacts)?;
        Ok(PublishedListing {
            product_id: product.id(),
            variant_id,
            title: title.to_owned(),
            price_minor,
        })
    }
}
