use axum::{body::Body, http::Request};
use market_bot_api::app::build_app;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn should_echo_a_valid_request_id() {
    let request_id = Uuid::new_v4();
    let response = build_app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .header("X-Request-Id", request_id.to_string())
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should execute");

    assert_eq!(
        response
            .headers()
            .get("X-Request-Id")
            .expect("response should include request id")
            .to_str()
            .expect("request id should be valid ASCII"),
        request_id.to_string()
    );
}

#[tokio::test]
async fn should_generate_a_request_id_when_header_is_missing() {
    let response = build_app()
        .oneshot(
            Request::builder()
                .uri("/healthz")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("request should execute");

    let request_id = response
        .headers()
        .get("X-Request-Id")
        .expect("response should include generated request id")
        .to_str()
        .expect("request id should be valid ASCII");

    assert!(Uuid::parse_str(request_id).is_ok());
}
