use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use market_bot_ai_agent::{AiScope, PurchasePolicy};
use market_bot_api::app::{AppState, build_app_with_state};
use market_bot_shared::UserId;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn send_json(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
    extra_headers: &[(&str, &str)],
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    for (name, value) in extra_headers {
        builder = builder.header(*name, *value);
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    let request = builder
        .body(match body {
            Some(value) => Body::from(serde_json::to_vec(&value).expect("json body should encode")),
            None => Body::empty(),
        })
        .expect("request should build");
    let response = app
        .clone()
        .oneshot(request)
        .await
        .expect("request should execute");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("response body should be readable")
        .to_bytes();
    let json = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).expect("response should be JSON")
    };
    (status, json)
}

fn auth_header(authorization_id: impl ToString) -> (String, String) {
    (
        "X-Ai-Authorization-Id".to_owned(),
        authorization_id.to_string(),
    )
}

fn allowed_policy() -> PurchasePolicy {
    PurchasePolicy::new(["electronics"], 10_000, 20_000, 50_000, 1_500, 80, true)
        .expect("policy should be valid")
}

#[tokio::test]
async fn should_search_products_when_authorization_has_catalog_read() {
    let state = AppState::default();
    let user_id = UserId::new();
    let authorization = state
        .ai
        .authorizations
        .authorize(
            user_id,
            "client-1",
            [AiScope::CatalogRead],
            Utc::now() + Duration::hours(1),
        )
        .await
        .expect("authorization should be created");
    let listing = state
        .ai
        .publish_listing("Portable Lamp", 2_500, 400, 3, 90)
        .await
        .expect("listing should publish");
    let app = build_app_with_state(state);
    let auth = auth_header(authorization.id());

    let (status, body) = send_json(
        &app,
        "GET",
        "/api/v1/ai/products/search?q=Lamp",
        None,
        &[(&auth.0, &auth.1)],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let items = body["items"].as_array().expect("items should be present");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["product_id"], listing.product_id.to_string());
    assert_eq!(items[0]["variant_id"], listing.variant_id.to_string());
    assert_eq!(items[0]["title"], "Portable Lamp");
    assert_eq!(items[0]["price_minor"], 2_500);
    assert!(body.get("next_cursor").is_some());
}

#[tokio::test]
async fn should_forbid_add_to_cart_without_cart_write() {
    let state = AppState::default();
    let user_id = UserId::new();
    let authorization = state
        .ai
        .authorizations
        .authorize(
            user_id,
            "client-1",
            [AiScope::CatalogRead],
            Utc::now() + Duration::hours(1),
        )
        .await
        .expect("authorization should be created");
    let listing = state
        .ai
        .publish_listing("Portable Lamp", 2_500, 400, 3, 90)
        .await
        .expect("listing should publish");
    let cart = state
        .ai
        .agent
        .cart_service()
        .create_cart(user_id)
        .await
        .expect("cart should be created");
    let app = build_app_with_state(state);
    let auth = auth_header(authorization.id());

    let (status, body) = send_json(
        &app,
        "POST",
        &format!("/api/v1/ai/carts/{}/items", cart.id()),
        Some(json!({
            "product_id": listing.product_id,
            "variant_id": listing.variant_id,
            "quantity": 1
        })),
        &[(&auth.0, &auth.1)],
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "FORBIDDEN");
}

#[tokio::test]
async fn should_forbid_auto_purchase_without_auto_purchase_scope() {
    let state = AppState::default();
    let user_id = UserId::new();
    let authorization = state
        .ai
        .authorizations
        .authorize(
            user_id,
            "client-1",
            [AiScope::OrderCreate],
            Utc::now() + Duration::hours(1),
        )
        .await
        .expect("authorization should be created");
    state
        .ai
        .authorizations
        .save_policy(user_id, allowed_policy())
        .await
        .expect("policy should save");
    state
        .ai
        .authorizations
        .set_auto_purchase_enabled(user_id, true)
        .await
        .expect("flag should save");
    let listing = state
        .ai
        .publish_listing("Portable Lamp", 2_500, 400, 3, 90)
        .await
        .expect("listing should publish");
    let app = build_app_with_state(state);
    let auth = auth_header(authorization.id());

    let (status, body) = send_json(
        &app,
        "POST",
        "/api/v1/ai/orders",
        Some(json!({
            "product_id": listing.product_id,
            "variant_id": listing.variant_id,
            "quantity": 1,
            "quoted_unit_price_minor": 2_500,
            "quoted_shipping_minor": 400
        })),
        &[(&auth.0, &auth.1), ("Idempotency-Key", "ai-order-no-scope")],
    )
    .await;

    assert_eq!(status, StatusCode::FORBIDDEN);
    assert_eq!(body["code"], "FORBIDDEN");
}

#[tokio::test]
async fn should_auto_purchase_when_scope_and_policy_allow() {
    let state = AppState::default();
    let user_id = UserId::new();
    let authorization = state
        .ai
        .authorizations
        .authorize(
            user_id,
            "client-1",
            [AiScope::AutoPurchase, AiScope::OrderRead],
            Utc::now() + Duration::hours(1),
        )
        .await
        .expect("authorization should be created");
    state
        .ai
        .authorizations
        .save_policy(user_id, allowed_policy())
        .await
        .expect("policy should save");
    state
        .ai
        .authorizations
        .set_auto_purchase_enabled(user_id, true)
        .await
        .expect("flag should save");
    let listing = state
        .ai
        .publish_listing("Portable Lamp", 2_500, 400, 3, 90)
        .await
        .expect("listing should publish");
    let app = build_app_with_state(state);
    let auth = auth_header(authorization.id());

    let (status, body) = send_json(
        &app,
        "POST",
        "/api/v1/ai/orders",
        Some(json!({
            "product_id": listing.product_id,
            "variant_id": listing.variant_id,
            "quantity": 1,
            "quoted_unit_price_minor": 2_500,
            "quoted_shipping_minor": 400
        })),
        &[(&auth.0, &auth.1), ("Idempotency-Key", "ai-order-ok")],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert!(body["order_id"].as_str().is_some());
    assert_eq!(body["total_minor"], 2_900);
    assert_eq!(body["currency"], "USD");
    assert!(body.get("payment_token").is_none());
    assert!(body.get("client_secret").is_none());

    let order_id = body["order_id"]
        .as_str()
        .expect("order_id should be present");
    let (get_status, order) = send_json(
        &app,
        "GET",
        &format!("/api/v1/ai/orders/{order_id}"),
        None,
        &[(&auth.0, &auth.1)],
    )
    .await;
    assert_eq!(get_status, StatusCode::OK);
    assert_eq!(order["order_id"], order_id);
    assert_eq!(order["items"].as_array().map(Vec::len), Some(1));
}
