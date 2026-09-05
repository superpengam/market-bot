use market_bot_shared::{CurrencyCode, Money, OrderId};

use super::{
    PaymentIntentInput, PaymentProvider, ProviderError, SANDBOX_SIGNATURE_HEADER,
    SandboxPaymentProvider, WebhookHeaders,
};

fn amount() -> Money {
    Money::new(
        2_500,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("amount should be valid")
}

#[tokio::test]
async fn should_create_a_sandbox_payment_intent() {
    let provider = SandboxPaymentProvider::new("test-secret");
    let order_id = OrderId::new();
    let intent = provider
        .create_payment_intent(PaymentIntentInput {
            order_id,
            amount: amount(),
        })
        .await
        .expect("sandbox intent should be created");

    assert_eq!(intent.order_id(), order_id);
    assert_eq!(intent.amount().minor(), 2_500);
    assert_eq!(intent.provider_payment_id(), format!("sandbox_{order_id}"));
}

#[test]
fn should_verify_a_signed_sandbox_webhook() {
    let provider = SandboxPaymentProvider::new("test-secret");
    let payload = br#"{"event_id":"evt-1","payment_id":"00000000-0000-0000-0000-000000000001","order_id":"00000000-0000-0000-0000-000000000002","kind":"PaymentSucceeded","occurred_at":"2026-09-03T12:00:00Z"}"#;
    let headers = provider.signed_headers(payload);

    let event = provider
        .verify_webhook(&headers, payload)
        .expect("signed webhook should verify");

    assert_eq!(event.event_id, "evt-1");
}

#[test]
fn should_reject_a_webhook_with_an_invalid_signature() {
    let provider = SandboxPaymentProvider::new("test-secret");
    let payload = br#"{"event_id":"evt-1","payment_id":"00000000-0000-0000-0000-000000000001","order_id":"00000000-0000-0000-0000-000000000002","kind":"PaymentSucceeded","occurred_at":"2026-09-03T12:00:00Z"}"#;
    let mut headers = WebhookHeaders::new();
    headers.insert(SANDBOX_SIGNATURE_HEADER, "invalid");

    assert_eq!(
        provider.verify_webhook(&headers, payload),
        Err(ProviderError::InvalidSignature)
    );
}
