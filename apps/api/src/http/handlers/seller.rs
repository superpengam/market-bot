use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use market_bot_catalog::{CatalogError, CreateProductCommand};
use market_bot_seller::{Store, StoreError};
use market_bot_shared::{
    ApiError, ErrorCode, ProductVariantId, RequestContext, SellerId, StoreId, UserId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::{AppState, ProductListingRecord},
    http::handlers::{
        ApiFulfillmentType, actor_user_id, error_response, invalid_input,
        product::{ProductDetailResponse, build_product_detail},
    },
};

#[derive(Debug, Deserialize)]
pub struct CreateStoreRequest {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct StoreResponse {
    pub store_id: StoreId,
    pub owner_id: UserId,
    pub seller_id: SellerId,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSellerProductRequest {
    pub title: String,
    pub description: String,
    pub fulfillment_type: ApiFulfillmentType,
    pub price_minor: i64,
    pub currency: String,
    pub available_stock: Option<u64>,
    pub refund_window_days: Option<u32>,
    pub digital: Option<DigitalDraftRequest>,
    pub physical: Option<PhysicalDraftRequest>,
}

#[derive(Debug, Deserialize)]
pub struct DigitalDraftRequest {
    pub delivery_method: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PhysicalDraftRequest {
    pub shipping_regions: Option<Vec<String>>,
    pub estimated_days_min: Option<u32>,
    pub estimated_days_max: Option<u32>,
}

pub async fn create_store(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(body): Json<CreateStoreRequest>,
) -> Result<Json<StoreResponse>, (StatusCode, Json<ApiError>)> {
    let owner_id = actor_user_id(&headers, context.request_id())?;
    let seller = state.directory.ensure_seller(owner_id).await;
    let store = Store::create(owner_id, body.name)
        .map_err(|error| map_store_error(error, context.request_id()))?;
    let store = state.directory.create_store(store).await;

    Ok(Json(StoreResponse {
        store_id: store.id(),
        owner_id: store.owner_id(),
        seller_id: seller.id(),
        name: store.name().to_owned(),
        slug: store.slug().to_owned(),
    }))
}

pub async fn create_product(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(body): Json<CreateSellerProductRequest>,
) -> Result<Json<ProductDetailResponse>, (StatusCode, Json<ApiError>)> {
    let owner_id = actor_user_id(&headers, context.request_id())?;
    let seller = state.directory.ensure_seller(owner_id).await;
    let store = state.directory.store_for_owner(owner_id).await;
    let product = state
        .catalog
        .create_product(CreateProductCommand {
            seller_id: seller.id(),
            title: body.title,
            description: body.description,
            product_type: body.fulfillment_type.into(),
            price_minor: body.price_minor,
            currency: body.currency,
        })
        .await
        .map_err(|error| map_catalog_error(error, context.request_id()))?;

    let variant_id = ProductVariantId::new();
    let available_stock = body.available_stock.unwrap_or(1);
    state
        .catalog
        .initialize_inventory(variant_id, available_stock)
        .await
        .map_err(|error| map_catalog_error(error, context.request_id()))?;

    let (estimated_days_min, estimated_days_max, digital_method, shipping_regions) =
        match body.fulfillment_type {
            ApiFulfillmentType::Digital => (
                0,
                0,
                Some(
                    body.digital
                        .as_ref()
                        .and_then(|draft| draft.delivery_method.clone())
                        .unwrap_or_else(|| "file_download".to_owned()),
                ),
                Vec::new(),
            ),
            ApiFulfillmentType::PhysicalStandard => (
                body.physical
                    .as_ref()
                    .and_then(|draft| draft.estimated_days_min)
                    .unwrap_or(3),
                body.physical
                    .as_ref()
                    .and_then(|draft| draft.estimated_days_max)
                    .unwrap_or(7),
                None,
                body.physical
                    .as_ref()
                    .and_then(|draft| draft.shipping_regions.clone())
                    .unwrap_or_default(),
            ),
        };

    state
        .directory
        .remember_listing(
            product.id(),
            ProductListingRecord {
                variant_id,
                store_id: store.as_ref().map(Store::id),
                store_name: store
                    .as_ref()
                    .map(|value| value.name().to_owned())
                    .unwrap_or_else(|| "Store".to_owned()),
                refund_window_days: body.refund_window_days.unwrap_or(14),
                digital_method,
                shipping_regions,
                estimated_days_min,
                estimated_days_max,
            },
        )
        .await;

    Ok(Json(
        build_product_detail(&state, &product, context.request_id()).await?,
    ))
}

pub async fn patch_product(
    Extension(context): Extension<RequestContext>,
    Path(_product_id): Path<Uuid>,
) -> (StatusCode, Json<ApiError>) {
    error_response(
        StatusCode::NOT_IMPLEMENTED,
        ErrorCode::InternalError,
        "seller product updates are not implemented",
        context.request_id(),
    )
}

fn map_store_error(error: StoreError, request_id: Uuid) -> (StatusCode, Json<ApiError>) {
    invalid_input(request_id, error.to_string())
}

fn map_catalog_error(error: CatalogError, request_id: Uuid) -> (StatusCode, Json<ApiError>) {
    let (status, code) = match error {
        CatalogError::InvalidProduct(_)
        | CatalogError::InvalidPrice(_)
        | CatalogError::InvalidCurrencyCode(_) => {
            (StatusCode::BAD_REQUEST, ErrorCode::InvalidInput)
        }
        CatalogError::Repository(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError)
        }
    };
    error_response(status, code, error.to_string(), request_id)
}
