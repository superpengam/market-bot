use axum::{body::Body, http::Request};
use http_body_util::BodyExt;
use market_bot_api::app::build_app;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn should_return_a_paginated_product_search_response() {
    let response = build_app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/products/search?q=lamp")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should execute");

    assert_eq!(response.status(), 200);
    let body = response
        .into_body()
        .collect()
        .await
        .expect("response body should be readable")
        .to_bytes();
    let json: Value = serde_json::from_slice(&body).expect("response should be JSON");

    assert!(json.get("items").is_some());
    assert!(json.get("next_cursor").is_some());
}
