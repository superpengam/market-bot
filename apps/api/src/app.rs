use std::collections::HashMap;
use std::sync::Arc;

use axum::{Json, Router, middleware, routing::get};
use market_bot_ai_agent::InMemoryAiStack;
use market_bot_cart::{CartService, InMemoryCartRepository};
use market_bot_catalog::{CatalogService, InMemoryCatalogRepository};
use market_bot_moderation::{
    InMemoryModerationRepository, ModerationService, SandboxContentScanner,
};
use market_bot_order::{InMemoryOrderRepository, OrderService};
use market_bot_payment::{InMemoryPaymentStore, PaymentEventHandler, SandboxPaymentProvider};
use market_bot_search::{InMemorySearchRepository, SearchService};
use market_bot_seller::{SellerProfile, Store};
use market_bot_shared::{InMemoryOutboxStore, ProductId, ProductVariantId, StoreId, UserId};
use serde::Serialize;
use tokio::sync::RwLock;

use crate::{http::routes::api_router, middleware::request_id::request_context_middleware};

pub type InMemoryModerationService = ModerationService<
    InMemoryModerationRepository,
    InMemoryCatalogRepository,
    InMemorySearchRepository,
    InMemoryOutboxStore,
    SandboxContentScanner,
>;

#[derive(Clone, Debug)]
pub struct ProductListingRecord {
    pub variant_id: ProductVariantId,
    pub store_id: Option<StoreId>,
    pub store_name: String,
    pub refund_window_days: u32,
    pub digital_method: Option<String>,
    pub shipping_regions: Vec<String>,
    pub estimated_days_min: u32,
    pub estimated_days_max: u32,
}

#[derive(Default)]
struct DirectoryState {
    sellers_by_owner: HashMap<UserId, SellerProfile>,
    stores_by_id: HashMap<StoreId, Store>,
    store_id_by_owner: HashMap<UserId, StoreId>,
    listings: HashMap<ProductId, ProductListingRecord>,
}

/// In-memory seller, store, and listing projection used by HTTP adapters.
#[derive(Clone, Default)]
pub struct MarketplaceDirectory {
    state: Arc<RwLock<DirectoryState>>,
}

impl MarketplaceDirectory {
    pub async fn ensure_seller(&self, owner_id: UserId) -> SellerProfile {
        let mut state = self.state.write().await;
        state
            .sellers_by_owner
            .entry(owner_id)
            .or_insert_with(|| SellerProfile::create(owner_id))
            .clone()
    }

    pub async fn create_store(&self, store: Store) -> Store {
        let mut state = self.state.write().await;
        state.store_id_by_owner.insert(store.owner_id(), store.id());
        state.stores_by_id.insert(store.id(), store.clone());
        store
    }

    pub async fn store_for_owner(&self, owner_id: UserId) -> Option<Store> {
        let state = self.state.read().await;
        let store_id = state.store_id_by_owner.get(&owner_id).copied()?;
        state.stores_by_id.get(&store_id).cloned()
    }

    pub async fn remember_listing(&self, product_id: ProductId, record: ProductListingRecord) {
        self.state.write().await.listings.insert(product_id, record);
    }

    pub async fn listing(&self, product_id: ProductId) -> Option<ProductListingRecord> {
        self.state.read().await.listings.get(&product_id).cloned()
    }

    pub async fn variant_for(&self, product_id: ProductId) -> Option<ProductVariantId> {
        self.state
            .read()
            .await
            .listings
            .get(&product_id)
            .map(|record| record.variant_id)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub search: SearchService<InMemorySearchRepository>,
    pub catalog: CatalogService<InMemoryCatalogRepository>,
    pub cart: CartService<InMemoryCartRepository>,
    pub orders: OrderService<InMemoryOrderRepository>,
    pub directory: MarketplaceDirectory,
    pub moderation: InMemoryModerationService,
    pub payment_handler: PaymentEventHandler<InMemoryPaymentStore, InMemoryOutboxStore>,
    pub payment_provider: SandboxPaymentProvider,
    pub payment_store: InMemoryPaymentStore,
    pub outbox: InMemoryOutboxStore,
    /// AI HTTP routes use this stack's agent, cart, and order services — not
    /// `cart` / `orders` above — so authorization checks and audit writes stay
    /// on the same in-memory collaborators.
    pub ai: InMemoryAiStack,
}

impl AppState {
    pub fn ai_stack(&self) -> &InMemoryAiStack {
        &self.ai
    }

    /// Builds a fully wired in-memory state around a caller-provided search
    /// repository so catalog, moderation and search observe the same listings.
    pub fn from_search_repository(repository: InMemorySearchRepository) -> Self {
        Self::build(
            repository,
            InMemoryPaymentStore::default(),
            InMemoryOutboxStore::default(),
            SandboxPaymentProvider::new("sandbox-dev-secret"),
        )
    }

    /// Builds in-memory state around caller-provided payment collaborators so
    /// webhook tests can inspect the payment store and outbox after requests.
    pub fn with_payment(
        payment_store: InMemoryPaymentStore,
        outbox: InMemoryOutboxStore,
        payment_provider: SandboxPaymentProvider,
    ) -> Self {
        Self::build(
            InMemorySearchRepository::default(),
            payment_store,
            outbox,
            payment_provider,
        )
    }

    fn build(
        search_repository: InMemorySearchRepository,
        payment_store: InMemoryPaymentStore,
        outbox: InMemoryOutboxStore,
        payment_provider: SandboxPaymentProvider,
    ) -> Self {
        let catalog_repository = InMemoryCatalogRepository::default();
        Self {
            search: SearchService::new(search_repository.clone()),
            catalog: CatalogService::new(catalog_repository.clone()),
            cart: CartService::new(InMemoryCartRepository::default()),
            orders: OrderService::new(InMemoryOrderRepository::default()),
            directory: MarketplaceDirectory::default(),
            moderation: ModerationService::new(
                InMemoryModerationRepository::default(),
                catalog_repository,
                search_repository,
                outbox.clone(),
                SandboxContentScanner,
            ),
            payment_handler: PaymentEventHandler::new(payment_store.clone(), outbox.clone()),
            payment_provider,
            payment_store,
            outbox,
            ai: InMemoryAiStack::new(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::from_search_repository(InMemorySearchRepository::default())
    }
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

/// Builds the API router with a caller-provided application state.
///
/// Keeping state injection explicit lets tests use an in-memory search
/// repository while production can provide external adapters later.
pub fn build_app_with_state(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(health))
        .merge(api_router())
        .layer(middleware::from_fn(request_context_middleware))
        .with_state(state)
}

/// Builds the public API router with empty in-memory state.
///
/// The router is kept as a pure function so integration tests can exercise the
/// HTTP contract without binding a real network port.
pub fn build_app() -> Router {
    build_app_with_state(AppState::default())
}
