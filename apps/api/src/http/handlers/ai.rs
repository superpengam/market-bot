use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use market_bot_ai_agent::{
    AddToCartInput, AiError, AiScope, AutoPurchaseInput, SearchProductsInput,
};
use market_bot_shared::{
    AiAuthorizationId, ApiError, CartId, ErrorCode, OrderId, Page, ProductId, ProductVariantId,
    RequestContext,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::AppState,
    http::handlers::{
        cart::{CartItemResponse, CartResponse},
        error_response, internal_error, invalid_input, not_found,
        order::{OrderDetailResponse, OrderLineItemResponse},
    },
};

const AI_AUTHORIZATION_HEADER: &str = "x-ai-authorization-id";

#[derive(Debug, Deserialize)]
pub struct AiProductSearchParams {
    pub q: Option<String>,
    pub category_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AiProductSearchItemResponse {
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub title: String,
    pub category_id: String,
    pub price_minor: i64,
    pub available_stock: u64,
}

#[derive(Debug, Deserialize)]
pub struct AiAddCartItemRequest {
    pub product_id: Uuid,
    pub variant_id: Uuid,
    pub quantity: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct AiAutoPurchaseRequest {
    pub product_id: Uuid,
    pub variant_id: Uuid,
    pub quantity: Option<u64>,
    pub quoted_unit_price_minor: i64,
    pub quoted_shipping_minor: i64,
}

pub async fn search_ai_products(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Query(params): Query<AiProductSearchParams>,
) -> Result<Json<Page<AiProductSearchItemResponse>>, (StatusCode, Json<ApiError>)> {
    let authorization_id = ai_authorization_id(&headers, context.request_id())?;
    let results = state
        .ai
        .agent
        .search_products(SearchProductsInput {
            authorization_id,
            query: params.q,
            category_id: params.category_id,
            request_id: context.request_id(),
        })
        .await
        .map_err(|error| map_ai_error(error, context.request_id()))?;

    Ok(Json(Page::new(
        results
            .into_iter()
            .map(|item| AiProductSearchItemResponse {
                product_id: item.product_id,
                variant_id: item.variant_id,
                title: item.title,
                category_id: item.category_id,
                price_minor: item.price_minor,
                available_stock: item.available_stock,
            })
            .collect(),
        None,
    )))
}

pub async fn add_ai_cart_item(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Path(cart_id): Path<Uuid>,
    Json(body): Json<AiAddCartItemRequest>,
) -> Result<Json<CartResponse>, (StatusCode, Json<ApiError>)> {
    let authorization_id = ai_authorization_id(&headers, context.request_id())?;
    let cart_id = CartId::from_uuid(cart_id);
    state
        .ai
        .agent
        .add_to_cart(AddToCartInput {
            authorization_id,
            cart_id,
            product_id: ProductId::from_uuid(body.product_id),
            variant_id: ProductVariantId::from_uuid(body.variant_id),
            quantity: body.quantity.unwrap_or(1),
            request_id: context.request_id(),
        })
        .await
        .map_err(|error| map_ai_error(error, context.request_id()))?;

    let cart = state
        .ai
        .agent
        .cart_service()
        .get_cart(cart_id)
        .await
        .map_err(|error| internal_error(context.request_id(), error.to_string()))?
        .ok_or_else(|| not_found(context.request_id(), "cart was not found"))?;

    let mut items = Vec::with_capacity(cart.items().len());
    for item in cart.items() {
        let available_stock = state
            .ai
            .catalog
            .get_inventory(item.variant_id())
            .await
            .map_err(|error| internal_error(context.request_id(), error.to_string()))?
            .map(|inventory| inventory.available_stock())
            .unwrap_or(0);
        items.push(CartItemResponse {
            cart_item_id: item.id(),
            product_id: item.product_id(),
            variant_id: item.variant_id(),
            title: item.title().to_owned(),
            unit_price_minor: item.unit_price().minor(),
            currency: item.unit_price().currency().as_str().to_owned(),
            quantity: item.quantity(),
            source: item.source().into(),
            fulfillment_type: item.fulfillment_type().into(),
            available_stock,
        });
    }

    Ok(Json(CartResponse {
        cart_id: cart.id(),
        items,
    }))
}

pub async fn create_ai_order(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(body): Json<AiAutoPurchaseRequest>,
) -> Result<Json<OrderDetailResponse>, (StatusCode, Json<ApiError>)> {
    let authorization_id = ai_authorization_id(&headers, context.request_id())?;
    let idempotency_key = context
        .idempotency_key()
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let order = state
        .ai
        .agent
        .auto_purchase(AutoPurchaseInput {
            authorization_id,
            product_id: ProductId::from_uuid(body.product_id),
            variant_id: ProductVariantId::from_uuid(body.variant_id),
            quantity: body.quantity.unwrap_or(1),
            quoted_unit_price_minor: body.quoted_unit_price_minor,
            quoted_shipping_minor: body.quoted_shipping_minor,
            idempotency_key,
            request_id: context.request_id(),
            now: Utc::now(),
        })
        .await
        .map_err(|error| map_ai_error(error, context.request_id()))?;

    Ok(Json(ai_order_response(&order)))
}

pub async fn get_ai_order(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OrderDetailResponse>, (StatusCode, Json<ApiError>)> {
    let authorization_id = ai_authorization_id(&headers, context.request_id())?;
    let authorization = state
        .ai
        .authorizations
        .require_scope(authorization_id, AiScope::OrderRead)
        .await
        .map_err(|error| map_ai_error(error, context.request_id()))?;
    let order = state
        .ai
        .agent
        .order_service()
        .get_order(OrderId::from_uuid(order_id))
        .await
        .map_err(|error| internal_error(context.request_id(), error.to_string()))?
        .ok_or_else(|| not_found(context.request_id(), "order was not found"))?;
    if order.buyer_id() != authorization.subject_user_id() {
        return Err(not_found(context.request_id(), "order was not found"));
    }

    Ok(Json(ai_order_response(&order)))
}

fn ai_authorization_id(
    headers: &HeaderMap,
    request_id: Uuid,
) -> Result<AiAuthorizationId, (StatusCode, Json<ApiError>)> {
    let Some(header) = headers.get(AI_AUTHORIZATION_HEADER) else {
        return Err(error_response(
            StatusCode::UNAUTHORIZED,
            ErrorCode::Unauthorized,
            "X-Ai-Authorization-Id is required",
            request_id,
        ));
    };
    let value = header
        .to_str()
        .map_err(|_| invalid_input(request_id, "X-Ai-Authorization-Id must be a UUID"))?;
    let authorization_id = Uuid::parse_str(value)
        .map_err(|_| invalid_input(request_id, "X-Ai-Authorization-Id must be a UUID"))?;
    Ok(AiAuthorizationId::from_uuid(authorization_id))
}

fn ai_order_response(order: &market_bot_ai_agent::Order) -> OrderDetailResponse {
    OrderDetailResponse {
        order_id: order.id(),
        order_status: order.status().into(),
        payment_status: "created",
        fulfillment_status: "pending",
        shipment_status: None,
        items: order
            .items()
            .iter()
            .map(|item| OrderLineItemResponse {
                order_item_id: item.id(),
                product_id: item.product_id(),
                variant_id: item.variant_id(),
                title: item.title().to_owned(),
                quantity: item.quantity(),
                unit_price_minor: item.unit_price().minor(),
                currency: item.unit_price().currency().as_str().to_owned(),
                fulfillment_type: item.fulfillment_type().into(),
            })
            .collect(),
        subtotal_minor: order.subtotal().minor(),
        shipping_fee_minor: order.shipping_fee().minor(),
        tax_minor: order.tax().minor(),
        total_minor: order.total().minor(),
        currency: order.total().currency().as_str().to_owned(),
        created_at: Utc::now(),
    }
}

fn map_ai_error(error: AiError, request_id: Uuid) -> (StatusCode, Json<ApiError>) {
    let status = match &error {
        AiError::AuthorizationNotFound | AiError::ProductNotFound | AiError::CartNotFound => {
            StatusCode::NOT_FOUND
        }
        AiError::AuthorizationExpired
        | AiError::AuthorizationRevoked
        | AiError::MissingScope { .. }
        | AiError::AutoPurchaseDisabled
        | AiError::CartOwnerMismatch
        | AiError::ProductNotPurchasable
        | AiError::PolicyNotFound
        | AiError::PolicyBlocked { .. } => StatusCode::FORBIDDEN,
        AiError::RequiresUserConfirmation { .. } | AiError::ProductOutOfStock => {
            StatusCode::CONFLICT
        }
        AiError::UnknownScope
        | AiError::BlankClientId
        | AiError::InvalidQuantity
        | AiError::BlankIdempotencyKey
        | AiError::InvalidPolicyAmount => StatusCode::BAD_REQUEST,
        AiError::Cart(_)
        | AiError::Order(_)
        | AiError::Catalog(_)
        | AiError::CatalogFacts(_)
        | AiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    };
    error_response(status, error.error_code(), error.to_string(), request_id)
}
