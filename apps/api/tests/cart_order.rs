use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use market_bot_api::app::build_app;
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

async fn seed_product(app: &axum::Router) -> (String, String) {
    let (store_status, _) = send_json(
        app,
        "POST",
        "/api/v1/seller/stores",
        Some(json!({ "name": "North Star Goods" })),
        &[],
    )
    .await;
    assert_eq!(store_status, StatusCode::OK);

    let (status, product) = send_json(
        app,
        "POST",
        "/api/v1/seller/products",
        Some(json!({
            "title": "Portable Lamp",
            "description": "A rechargeable lamp",
            "fulfillment_type": "physical_standard",
            "price_minor": 1999,
            "currency": "USD",
            "available_stock": 8
        })),
        &[],
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let product_id = product["product_id"]
        .as_str()
        .expect("product_id should be present")
        .to_owned();
    let variant_id = product["variants"][0]["variant_id"]
        .as_str()
        .expect("variant_id should be present")
        .to_owned();
    (product_id, variant_id)
}

async fn seed_cart_with_item(app: &axum::Router) -> (String, String, String) {
    let (product_id, variant_id) = seed_product(app).await;
    let (cart_status, cart) = send_json(app, "POST", "/api/v1/carts", None, &[]).await;
    assert_eq!(cart_status, StatusCode::OK);
    let cart_id = cart["cart_id"]
        .as_str()
        .expect("cart_id should be present")
        .to_owned();

    let (add_status, added) = send_json(
        app,
        "POST",
        &format!("/api/v1/carts/{cart_id}/items"),
        Some(json!({
            "product_id": product_id,
            "variant_id": variant_id,
            "quantity": 2,
            "source": "user"
        })),
        &[],
    )
    .await;
    assert_eq!(add_status, StatusCode::OK);
    assert_eq!(added["items"].as_array().map(Vec::len), Some(1));
    (product_id, variant_id, cart_id)
}

#[tokio::test]
async fn should_create_cart_add_item_and_get_cart() {
    let app = build_app();
    let (product_id, variant_id, cart_id) = seed_cart_with_item(&app).await;

    let (status, cart) =
        send_json(&app, "GET", &format!("/api/v1/carts/{cart_id}"), None, &[]).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(cart["cart_id"], cart_id);
    assert_eq!(cart["items"][0]["product_id"], product_id);
    assert_eq!(cart["items"][0]["variant_id"], variant_id);
    assert_eq!(cart["items"][0]["quantity"], 2);
    assert_eq!(cart["items"][0]["unit_price_minor"], 1999);
    assert_eq!(cart["items"][0]["currency"], "USD");
}

#[tokio::test]
async fn should_return_checkout_preview_totals() {
    let app = build_app();
    let (_, _, cart_id) = seed_cart_with_item(&app).await;

    let (status, preview) = send_json(
        &app,
        "POST",
        "/api/v1/checkout/preview",
        Some(json!({ "cart_id": cart_id })),
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(preview["subtotal_minor"], 3998);
    assert_eq!(preview["shipping_fee_minor"], 0);
    assert_eq!(preview["tax_minor"], 0);
    assert_eq!(preview["total_minor"], 3998);
    assert_eq!(preview["currency"], "USD");
    assert_eq!(preview["inventory_is_available"], true);
    assert_eq!(preview["requires_price_reconfirm"], false);
    assert!(preview["expires_at"].as_str().is_some());
}

#[tokio::test]
async fn should_reuse_the_same_order_id_for_a_repeated_idempotency_key() {
    let app = build_app();
    let (_, _, cart_id) = seed_cart_with_item(&app).await;
    let headers = [("Idempotency-Key", "checkout-lamp-1")];

    let (first_status, first) = send_json(
        &app,
        "POST",
        "/api/v1/orders",
        Some(json!({ "cart_id": cart_id })),
        &headers,
    )
    .await;
    let (second_status, second) = send_json(
        &app,
        "POST",
        "/api/v1/orders",
        Some(json!({ "cart_id": cart_id })),
        &headers,
    )
    .await;

    assert_eq!(first_status, StatusCode::OK);
    assert_eq!(second_status, StatusCode::OK);
    assert_eq!(first["order_id"], second["order_id"]);
    assert_eq!(first["order_status"], "pending_payment");
    assert_eq!(first["total_minor"], 3998);
}

#[tokio::test]
async fn should_get_order_by_id() {
    let app = build_app();
    let (_, _, cart_id) = seed_cart_with_item(&app).await;
    let (create_status, created) = send_json(
        &app,
        "POST",
        "/api/v1/orders",
        Some(json!({ "cart_id": cart_id })),
        &[("Idempotency-Key", "checkout-lamp-get")],
    )
    .await;
    assert_eq!(create_status, StatusCode::OK);
    let order_id = created["order_id"]
        .as_str()
        .expect("order_id should be present");

    let (status, order) = send_json(
        &app,
        "GET",
        &format!("/api/v1/orders/{order_id}"),
        None,
        &[],
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(order["order_id"], order_id);
    assert_eq!(order["items"].as_array().map(Vec::len), Some(1));
    assert_eq!(order["subtotal_minor"], 3998);
    assert_eq!(order["currency"], "USD");
}
