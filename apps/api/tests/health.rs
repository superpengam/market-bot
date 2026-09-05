use axum::{body::Body, http::Request};
use market_bot_api::app::build_app;
use tower::ServiceExt;

#[tokio::test]
async fn should_return_ok_from_health_endpoint() {
    let response = build_app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .expect("health request should build"),
        )
        .await
        .expect("health request should execute");

    assert_eq!(response.status(), 200);
}
