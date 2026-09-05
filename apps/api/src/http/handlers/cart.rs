use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use market_bot_cart::{AddCartItem, Cart, CartError, CartItem, CartServiceError};
use market_bot_shared::{
    ApiError, CartId, CartItemId, ErrorCode, ProductId, ProductVariantId, RequestContext,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::AppState,
    http::handlers::{
        ApiCartItemSource, ApiFulfillmentType, actor_user_id, error_response, internal_error,
        invalid_input, not_found,
    },
};

#[derive(Debug, Serialize)]
pub struct CartResponse {
    pub cart_id: CartId,
    pub items: Vec<CartItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct CartItemResponse {
    pub cart_item_id: CartItemId,
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub title: String,
    pub unit_price_minor: i64,
    pub currency: String,
    pub quantity: u64,
    pub source: ApiCartItemSource,
    pub fulfillment_type: ApiFulfillmentType,
    pub available_stock: u64,
}

#[derive(Debug, Deserialize)]
pub struct AddCartItemRequest {
    pub product_id: Uuid,
    pub variant_id: Option<Uuid>,
    pub quantity: Option<u64>,
    pub source: Option<ApiCartItemSource>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCartItemRequest {
    pub quantity: u64,
}

pub async fn create_cart(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
) -> Result<Json<CartResponse>, (StatusCode, Json<ApiError>)> {
    let owner_id = actor_user_id(&headers, context.request_id())?;
    let cart = state
        .cart
        .create_cart(owner_id)
        .await
        .map_err(|error| map_cart_error(error, context.request_id()))?;
    Ok(Json(
        cart_response(&state, &cart, context.request_id()).await?,
    ))
}

pub async fn get_cart(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(cart_id): Path<Uuid>,
) -> Result<Json<CartResponse>, (StatusCode, Json<ApiError>)> {
    let cart = load_cart(&state, CartId::from_uuid(cart_id), context.request_id()).await?;
    Ok(Json(
        cart_response(&state, &cart, context.request_id()).await?,
    ))
}

pub async fn add_item(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(cart_id): Path<Uuid>,
    Json(body): Json<AddCartItemRequest>,
) -> Result<Json<CartResponse>, (StatusCode, Json<ApiError>)> {
    let cart_id = CartId::from_uuid(cart_id);
    let product_id = ProductId::from_uuid(body.product_id);
    let product = state
        .catalog
        .get_product(product_id)
        .await
        .map_err(|error| internal_error(context.request_id(), error.to_string()))?
        .ok_or_else(|| not_found(context.request_id(), "product was not found"))?;
    let variant_id = match body.variant_id {
        Some(variant_id) => ProductVariantId::from_uuid(variant_id),
        None => state
            .directory
            .variant_for(product_id)
            .await
            .unwrap_or_else(|| ProductVariantId::from_uuid(product_id.as_uuid())),
    };

    state
        .cart
        .add_item(
            cart_id,
            AddCartItem {
                product_id,
                variant_id,
                title: product.title().to_owned(),
                unit_price: product.price().clone(),
                quantity: body.quantity.unwrap_or(1),
                source: body.source.unwrap_or(ApiCartItemSource::User).into(),
                fulfillment_type: product.product_type(),
            },
        )
        .await
        .map_err(|error| map_cart_error(error, context.request_id()))?;

    let cart = load_cart(&state, cart_id, context.request_id()).await?;
    Ok(Json(
        cart_response(&state, &cart, context.request_id()).await?,
    ))
}

pub async fn update_item(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path((cart_id, item_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateCartItemRequest>,
) -> Result<Json<CartResponse>, (StatusCode, Json<ApiError>)> {
    let cart_id = CartId::from_uuid(cart_id);
    state
        .cart
        .update_quantity(cart_id, CartItemId::from_uuid(item_id), body.quantity)
        .await
        .map_err(|error| map_cart_error(error, context.request_id()))?;
    let cart = load_cart(&state, cart_id, context.request_id()).await?;
    Ok(Json(
        cart_response(&state, &cart, context.request_id()).await?,
    ))
}

pub async fn remove_item(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path((cart_id, item_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let removed = state
        .cart
        .remove_item(CartId::from_uuid(cart_id), CartItemId::from_uuid(item_id))
        .await
        .map_err(|error| map_cart_error(error, context.request_id()))?;
    if !removed {
        return Err(not_found(context.request_id(), "cart item was not found"));
    }
    Ok(StatusCode::NO_CONTENT)
}

pub async fn load_cart(
    state: &AppState,
    cart_id: CartId,
    request_id: Uuid,
) -> Result<Cart, (StatusCode, Json<ApiError>)> {
    state
        .cart
        .get_cart(cart_id)
        .await
        .map_err(|error| map_cart_error(error, request_id))?
        .ok_or_else(|| not_found(request_id, "cart was not found"))
}

async fn cart_response(
    state: &AppState,
    cart: &Cart,
    request_id: Uuid,
) -> Result<CartResponse, (StatusCode, Json<ApiError>)> {
    let mut items = Vec::with_capacity(cart.items().len());
    for item in cart.items() {
        items.push(cart_item_response(state, item, request_id).await?);
    }
    Ok(CartResponse {
        cart_id: cart.id(),
        items,
    })
}

async fn cart_item_response(
    state: &AppState,
    item: &CartItem,
    request_id: Uuid,
) -> Result<CartItemResponse, (StatusCode, Json<ApiError>)> {
    let available_stock = state
        .catalog
        .get_inventory(item.variant_id())
        .await
        .map_err(|error| internal_error(request_id, error.to_string()))?
        .map(|inventory| inventory.available_stock())
        .unwrap_or(0);
    Ok(CartItemResponse {
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
    })
}

fn map_cart_error(error: CartServiceError, request_id: Uuid) -> (StatusCode, Json<ApiError>) {
    let (status, code) = match error {
        CartServiceError::CartNotFound => (StatusCode::NOT_FOUND, ErrorCode::NotFound),
        CartServiceError::ProductNotPurchasable => (StatusCode::FORBIDDEN, ErrorCode::Forbidden),
        CartServiceError::Cart(CartError::ItemNotFound) => {
            (StatusCode::NOT_FOUND, ErrorCode::NotFound)
        }
        CartServiceError::Cart(CartError::PriceSnapshotChanged) => {
            (StatusCode::CONFLICT, ErrorCode::PriceChanged)
        }
        CartServiceError::Cart(
            CartError::InvalidQuantity | CartError::BlankTitle | CartError::QuantityOverflow,
        ) => (StatusCode::BAD_REQUEST, ErrorCode::InvalidInput),
        CartServiceError::Repository(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError)
        }
    };
    error_response(status, code, error.to_string(), request_id)
}

pub fn empty_cart_error(request_id: Uuid) -> (StatusCode, Json<ApiError>) {
    invalid_input(request_id, "cart has no items")
}
