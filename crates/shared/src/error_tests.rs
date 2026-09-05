use uuid::Uuid;

use super::{ApiError, ErrorCode, RequestContext};

#[test]
fn should_include_stable_code_and_request_id_in_api_error() {
    let request_id = Uuid::new_v4();
    let error = ApiError::new(ErrorCode::InvalidInput, "title is required", request_id);

    assert_eq!(error.code().as_str(), "INVALID_INPUT");
    assert_eq!(error.message(), "title is required");
    assert_eq!(error.request_id(), request_id);
}

#[test]
fn should_preserve_idempotency_key_in_request_context() {
    let request_id = Uuid::new_v4();
    let context = RequestContext::new(request_id, Some("order-request-1".to_owned()))
        .expect("context should be valid");

    assert_eq!(context.request_id(), request_id);
    assert_eq!(context.idempotency_key(), Some("order-request-1"));
}

#[test]
fn should_reject_blank_idempotency_key() {
    let request_id = Uuid::new_v4();

    assert!(RequestContext::new(request_id, Some("   ".to_owned())).is_err());
}
