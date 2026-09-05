use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use market_bot_cart::Cart;
use market_bot_order::{
    CreateOrderCommand, Order, OrderError, OrderItem, OrderServiceError, OrderStatus,
};
use market_bot_shared::{
    ApiError, CartId, ErrorCode, Money, OrderId, ProductId, ProductVariantId, RequestContext,
    SellerId,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    app::AppState,
    http::handlers::{
        ApiFulfillmentType, ApiOrderStatus, actor_user_id,
        cart::{empty_cart_error, load_cart},
        error_response, internal_error, invalid_input, not_found,
    },
};

#[derive(Debug, Deserialize)]
pub struct CheckoutPreviewRequest {
    pub cart_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct CheckoutLineItemResponse {
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub title: String,
    pub quantity: u64,
    pub snapshot_unit_price_minor: i64,
    pub current_unit_price_minor: i64,
    pub currency: String,
    pub fulfillment_type: ApiFulfillmentType,
    pub available_stock: u64,
    pub source: crate::http::handlers::ApiCartItemSource,
}

#[derive(Debug, Serialize)]
pub struct CheckoutPreviewResponse {
    pub items: Vec<CheckoutLineItemResponse>,
    pub subtotal_minor: i64,
    pub shipping_fee_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub currency: String,
    pub expires_at: chrono::DateTime<Utc>,
    pub requires_price_reconfirm: bool,
    pub inventory_is_available: bool,
    pub payment_provider_status: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub cart_id: Option<Uuid>,
    pub items: Option<Vec<CreateOrderItemRequest>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderItemRequest {
    pub product_id: Uuid,
    pub variant_id: Uuid,
    pub quantity: u64,
    pub title: Option<String>,
    pub unit_price_minor: Option<i64>,
    pub currency: Option<String>,
    pub seller_id: Option<Uuid>,
    pub fulfillment_type: Option<ApiFulfillmentType>,
}

#[derive(Debug, Serialize)]
pub struct OrderLineItemResponse {
    pub order_item_id: market_bot_shared::OrderItemId,
    pub product_id: ProductId,
    pub variant_id: ProductVariantId,
    pub title: String,
    pub quantity: u64,
    pub unit_price_minor: i64,
    pub currency: String,
    pub fulfillment_type: ApiFulfillmentType,
}

#[derive(Debug, Serialize)]
pub struct OrderDetailResponse {
    pub order_id: OrderId,
    pub order_status: ApiOrderStatus,
    pub payment_status: &'static str,
    pub fulfillment_status: &'static str,
    pub shipment_status: Option<&'static str>,
    pub items: Vec<OrderLineItemResponse>,
    pub subtotal_minor: i64,
    pub shipping_fee_minor: i64,
    pub tax_minor: i64,
    pub total_minor: i64,
    pub currency: String,
    pub created_at: chrono::DateTime<Utc>,
}

pub async fn preview_checkout(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Json(body): Json<CheckoutPreviewRequest>,
) -> Result<Json<CheckoutPreviewResponse>, (StatusCode, Json<ApiError>)> {
    let cart = load_cart(
        &state,
        CartId::from_uuid(body.cart_id),
        context.request_id(),
    )
    .await?;
    if cart.items().is_empty() {
        return Err(empty_cart_error(context.request_id()));
    }

    Ok(Json(
        build_checkout_preview(&state, &cart, context.request_id()).await?,
    ))
}

pub async fn create_order(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(body): Json<CreateOrderRequest>,
) -> Result<Json<OrderDetailResponse>, (StatusCode, Json<ApiError>)> {
    let buyer_id = actor_user_id(&headers, context.request_id())?;
    let items = match body.cart_id {
        Some(cart_id) => {
            let cart = load_cart(&state, CartId::from_uuid(cart_id), context.request_id()).await?;
            order_items_from_cart(&state, &cart, context.request_id()).await?
        }
        None => {
            order_items_from_request(&state, body.items.unwrap_or_default(), context.request_id())
                .await?
        }
    };
    if items.is_empty() {
        return Err(invalid_input(
            context.request_id(),
            "order must contain at least one item",
        ));
    }

    let currency = items[0].unit_price().currency().clone();
    let shipping_fee = Money::new(0, currency.clone())
        .map_err(|error| invalid_input(context.request_id(), error.to_string()))?;
    let tax = Money::new(0, currency)
        .map_err(|error| invalid_input(context.request_id(), error.to_string()))?;
    let idempotency_key = context
        .idempotency_key()
        .map(str::to_owned)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let order = state
        .orders
        .create_order(CreateOrderCommand {
            buyer_id,
            items,
            shipping_fee,
            tax,
            idempotency_key,
        })
        .await
        .map_err(|error| map_order_error(error, context.request_id()))?;
    let order = advance_created_order(&state, order, context.request_id()).await?;

    Ok(Json(order_response(&order)))
}

pub async fn get_order(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OrderDetailResponse>, (StatusCode, Json<ApiError>)> {
    let order = state
        .orders
        .get_order(OrderId::from_uuid(order_id))
        .await
        .map_err(|error| map_order_error(error, context.request_id()))?
        .ok_or_else(|| not_found(context.request_id(), "order was not found"))?;
    Ok(Json(order_response(&order)))
}

pub async fn cancel_order(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(order_id): Path<Uuid>,
) -> Result<Json<OrderDetailResponse>, (StatusCode, Json<ApiError>)> {
    let order_id = OrderId::from_uuid(order_id);
    let order = state
        .orders
        .get_order(order_id)
        .await
        .map_err(|error| map_order_error(error, context.request_id()))?
        .ok_or_else(|| not_found(context.request_id(), "order was not found"))?;
    if order.status() == OrderStatus::Cancelled {
        return Ok(Json(order_response(&order)));
    }

    let order = state
        .orders
        .transition_order(order_id, OrderStatus::Cancelled)
        .await
        .map_err(|error| map_order_error(error, context.request_id()))?;
    Ok(Json(order_response(&order)))
}

async fn build_checkout_preview(
    state: &AppState,
    cart: &Cart,
    request_id: Uuid,
) -> Result<CheckoutPreviewResponse, (StatusCode, Json<ApiError>)> {
    let mut items = Vec::new();
    let mut subtotal_minor = 0_i64;
    let mut requires_price_reconfirm = false;
    let mut inventory_is_available = true;
    let mut currency = None;

    for item in cart.items() {
        let product = state
            .catalog
            .get_product(item.product_id())
            .await
            .map_err(|error| internal_error(request_id, error.to_string()))?
            .ok_or_else(|| not_found(request_id, "product was not found"))?;
        let available_stock = state
            .catalog
            .get_inventory(item.variant_id())
            .await
            .map_err(|error| internal_error(request_id, error.to_string()))?
            .map(|inventory| inventory.available_stock());
        let current_unit_price_minor = product.price().minor();
        if current_unit_price_minor != item.unit_price().minor() {
            requires_price_reconfirm = true;
        }
        if available_stock.is_some_and(|stock| stock < item.quantity()) {
            inventory_is_available = false;
        }
        let line_total = current_unit_price_minor
            .checked_mul(
                i64::try_from(item.quantity())
                    .map_err(|_| invalid_input(request_id, "cart item quantity is too large"))?,
            )
            .ok_or_else(|| invalid_input(request_id, "checkout subtotal overflowed"))?;
        subtotal_minor = subtotal_minor
            .checked_add(line_total)
            .ok_or_else(|| invalid_input(request_id, "checkout subtotal overflowed"))?;
        currency = Some(product.price().currency().as_str().to_owned());

        items.push(CheckoutLineItemResponse {
            product_id: item.product_id(),
            variant_id: item.variant_id(),
            title: product.title().to_owned(),
            quantity: item.quantity(),
            snapshot_unit_price_minor: item.unit_price().minor(),
            current_unit_price_minor,
            currency: product.price().currency().as_str().to_owned(),
            fulfillment_type: product.product_type().into(),
            available_stock: available_stock.unwrap_or(0),
            source: item.source().into(),
        });
    }

    let shipping_fee_minor = 0;
    let tax_minor = 0;
    Ok(CheckoutPreviewResponse {
        items,
        subtotal_minor,
        shipping_fee_minor,
        tax_minor,
        total_minor: subtotal_minor + shipping_fee_minor + tax_minor,
        currency: currency.unwrap_or_else(|| "USD".to_owned()),
        expires_at: Utc::now() + chrono::Duration::minutes(15),
        requires_price_reconfirm,
        inventory_is_available,
        payment_provider_status: "not_started",
    })
}

async fn order_items_from_cart(
    state: &AppState,
    cart: &Cart,
    request_id: Uuid,
) -> Result<Vec<OrderItem>, (StatusCode, Json<ApiError>)> {
    if cart.items().is_empty() {
        return Err(empty_cart_error(request_id));
    }

    let mut items = Vec::new();
    for item in cart.items() {
        let product = state
            .catalog
            .get_product(item.product_id())
            .await
            .map_err(|error| internal_error(request_id, error.to_string()))?
            .ok_or_else(|| not_found(request_id, "product was not found"))?;
        items.push(
            OrderItem::new(
                item.product_id(),
                item.variant_id(),
                product.seller_id(),
                product.title().to_owned(),
                product.price().clone(),
                item.quantity(),
                product.product_type(),
            )
            .map_err(|error| invalid_input(request_id, error.to_string()))?,
        );
    }
    Ok(items)
}

async fn order_items_from_request(
    state: &AppState,
    items: Vec<CreateOrderItemRequest>,
    request_id: Uuid,
) -> Result<Vec<OrderItem>, (StatusCode, Json<ApiError>)> {
    let mut order_items = Vec::new();
    for item in items {
        let product_id = ProductId::from_uuid(item.product_id);
        let product = state.catalog.get_product(product_id).await.ok().flatten();
        let title = item
            .title
            .or_else(|| product.as_ref().map(|value| value.title().to_owned()))
            .ok_or_else(|| invalid_input(request_id, "order item title is required"))?;
        let unit_price = if let Some(product) = product.as_ref() {
            product.price().clone()
        } else {
            let currency = item
                .currency
                .ok_or_else(|| invalid_input(request_id, "order item currency is required"))?;
            let currency = market_bot_shared::CurrencyCode::try_from(currency)
                .map_err(|error| invalid_input(request_id, error.to_string()))?;
            Money::new(item.unit_price_minor.unwrap_or(0), currency)
                .map_err(|error| invalid_input(request_id, error.to_string()))?
        };
        let seller_id = item
            .seller_id
            .map(SellerId::from_uuid)
            .or_else(|| product.as_ref().map(|value| value.seller_id()))
            .ok_or_else(|| invalid_input(request_id, "order item seller_id is required"))?;
        let fulfillment_type = item
            .fulfillment_type
            .map(Into::into)
            .or_else(|| product.as_ref().map(|value| value.product_type()))
            .ok_or_else(|| invalid_input(request_id, "order item fulfillment_type is required"))?;

        order_items.push(
            OrderItem::new(
                product_id,
                ProductVariantId::from_uuid(item.variant_id),
                seller_id,
                title,
                unit_price,
                item.quantity,
                fulfillment_type,
            )
            .map_err(|error| invalid_input(request_id, error.to_string()))?,
        );
    }
    Ok(order_items)
}

async fn advance_created_order(
    state: &AppState,
    order: Order,
    request_id: Uuid,
) -> Result<Order, (StatusCode, Json<ApiError>)> {
    let mut current = order;
    if current.status() == OrderStatus::Draft {
        current = state
            .orders
            .transition_order(current.id(), OrderStatus::PendingConfirmation)
            .await
            .map_err(|error| map_order_error(error, request_id))?;
    }
    if current.status() == OrderStatus::PendingConfirmation {
        current = state
            .orders
            .transition_order(current.id(), OrderStatus::PendingPayment)
            .await
            .map_err(|error| map_order_error(error, request_id))?;
    }
    Ok(current)
}

fn order_response(order: &Order) -> OrderDetailResponse {
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

fn map_order_error(error: OrderServiceError, request_id: Uuid) -> (StatusCode, Json<ApiError>) {
    let (status, code) = match error {
        OrderServiceError::BlankIdempotencyKey => {
            (StatusCode::BAD_REQUEST, ErrorCode::InvalidInput)
        }
        OrderServiceError::OrderNotFound => (StatusCode::NOT_FOUND, ErrorCode::NotFound),
        OrderServiceError::Order(OrderError::InvalidStatusTransition { .. }) => {
            (StatusCode::CONFLICT, ErrorCode::OrderStateInvalid)
        }
        OrderServiceError::Order(
            OrderError::EmptyOrder
            | OrderError::InvalidQuantity
            | OrderError::BlankTitle
            | OrderError::Money(_),
        ) => (StatusCode::BAD_REQUEST, ErrorCode::InvalidInput),
        OrderServiceError::Repository(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError)
        }
    };
    error_response(status, code, error.to_string(), request_id)
}
