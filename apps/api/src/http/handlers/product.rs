use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
};
use market_bot_catalog::Product;
use market_bot_shared::{ApiError, ProductId, ProductVariantId, RequestContext, SellerId, StoreId};
use serde::Serialize;
use uuid::Uuid;

use crate::{
    app::AppState,
    http::handlers::{
        ApiFulfillmentType, ApiProductStatus, error_response, internal_error, not_found,
    },
};

#[derive(Debug, Serialize)]
pub struct ProductVariantResponse {
    pub variant_id: ProductVariantId,
    pub sku: String,
    pub price_minor: i64,
    pub currency: String,
    pub available_stock: u64,
}

#[derive(Debug, Serialize)]
pub struct DeliveryRulesResponse {
    pub fulfillment_type: ApiFulfillmentType,
    pub estimated_days_min: u32,
    pub estimated_days_max: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub digital_method: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub shipping_regions: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct RefundRulesResponse {
    pub refund_window_days: u32,
    pub is_refundable: bool,
    pub summary: String,
}

#[derive(Debug, Serialize)]
pub struct ProductDetailResponse {
    pub product_id: ProductId,
    pub seller_id: SellerId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub store_id: Option<StoreId>,
    pub store_name: String,
    pub title: String,
    pub description: String,
    pub fulfillment_type: ApiFulfillmentType,
    pub price_minor: i64,
    pub currency: String,
    pub available_stock: u64,
    pub status: ApiProductStatus,
    pub delivery_rules: DeliveryRulesResponse,
    pub refund_rules: RefundRulesResponse,
    pub variants: Vec<ProductVariantResponse>,
}

pub async fn get_product(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(product_id): Path<Uuid>,
) -> Result<Json<ProductDetailResponse>, (StatusCode, Json<ApiError>)> {
    let product_id = ProductId::from_uuid(product_id);
    let product = state
        .catalog
        .get_product(product_id)
        .await
        .map_err(|error| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                market_bot_shared::ErrorCode::InternalError,
                error.to_string(),
                context.request_id(),
            )
        })?
        .ok_or_else(|| not_found(context.request_id(), "product was not found"))?;

    Ok(Json(
        build_product_detail(&state, &product, context.request_id()).await?,
    ))
}

pub async fn build_product_detail(
    state: &AppState,
    product: &Product,
    request_id: Uuid,
) -> Result<ProductDetailResponse, (StatusCode, Json<ApiError>)> {
    let listing = state.directory.listing(product.id()).await;
    let variant_id = listing
        .as_ref()
        .map(|record| record.variant_id)
        .unwrap_or_else(|| ProductVariantId::from_uuid(product.id().as_uuid()));
    let available_stock = state
        .catalog
        .get_inventory(variant_id)
        .await
        .map_err(|error| internal_error(request_id, error.to_string()))?
        .map(|inventory| inventory.available_stock())
        .unwrap_or(0);
    let fulfillment_type = ApiFulfillmentType::from(product.product_type());
    let refund_window_days = listing
        .as_ref()
        .map(|record| record.refund_window_days)
        .unwrap_or(14);
    let (estimated_days_min, estimated_days_max, digital_method, shipping_regions) =
        match listing.as_ref() {
            Some(record) => (
                record.estimated_days_min,
                record.estimated_days_max,
                record.digital_method.clone(),
                record.shipping_regions.clone(),
            ),
            None => match fulfillment_type {
                ApiFulfillmentType::Digital => (0, 0, Some("file_download".to_owned()), Vec::new()),
                ApiFulfillmentType::PhysicalStandard => (3, 7, None, Vec::new()),
            },
        };

    Ok(ProductDetailResponse {
        product_id: product.id(),
        seller_id: product.seller_id(),
        store_id: listing.as_ref().and_then(|record| record.store_id),
        store_name: listing
            .as_ref()
            .map(|record| record.store_name.clone())
            .unwrap_or_else(|| "Store".to_owned()),
        title: product.title().to_owned(),
        description: product.description().to_owned(),
        fulfillment_type,
        price_minor: product.price().minor(),
        currency: product.price().currency().as_str().to_owned(),
        available_stock,
        status: ApiProductStatus::from(product.status()),
        delivery_rules: DeliveryRulesResponse {
            fulfillment_type,
            estimated_days_min,
            estimated_days_max,
            digital_method,
            shipping_regions,
        },
        refund_rules: RefundRulesResponse {
            refund_window_days,
            is_refundable: refund_window_days > 0,
            summary: if refund_window_days > 0 {
                format!("{refund_window_days} day refund window.")
            } else {
                "Not refundable.".to_owned()
            },
        },
        variants: vec![ProductVariantResponse {
            variant_id,
            sku: format!("SKU-{}", &product.id().to_string()[..8]),
            price_minor: product.price().minor(),
            currency: product.price().currency().as_str().to_owned(),
            available_stock,
        }],
    })
}
