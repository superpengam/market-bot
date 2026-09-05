use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use market_bot_api::app::{AppState, build_app_with_state};
use market_bot_payment::{InMemoryPaymentStore, Payment, PaymentStatus, SandboxPaymentProvider};
use market_bot_shared::{
    CurrencyCode, InMemoryOutboxStore, Money, OrderId, OutboxEvent, OutboxStatus, OutboxStore,
};
use market_bot_worker::jobs::publish_outbox::{EventPublisher, OutboxPublisher};
use serde_json::{Value, json};
use tower::ServiceExt;

fn usd(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("amount should be valid")
}

async fn seed_payment(store: &InMemoryPaymentStore, amount_minor: i64) -> Payment {
    let payment = Payment::new(OrderId::new(), usd(amount_minor));
    store
        .save_payment(payment.clone())
        .await
        .expect("payment should be stored");
    payment
}

fn webhook_payload(
    event_id: &str,
    payment: &Payment,
    kind: &str,
    occurred_at: chrono::DateTime<Utc>,
) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "event_id": event_id,
        "payment_id": payment.id(),
        "order_id": payment.order_id(),
        "kind": kind,
        "occurred_at": occurred_at,
    }))
    .expect("webhook payload should serialize")
}

async fn post_webhook(state: AppState, signature: &str, body: Vec<u8>) -> (StatusCode, Value) {
    let response = build_app_with_state(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/payments/webhooks")
                .header("content-type", "application/json")
                .header("x-sandbox-signature", signature)
                .body(Body::from(body))
                .expect("webhook request should build"),
        )
        .await
        .expect("webhook request should execute");

    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("response body should be readable")
        .to_bytes();
    let json = if bytes.is_empty() {
        json!({})
    } else {
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }))
    };
    (status, json)
}

#[tokio::test]
async fn should_ignore_duplicate_provider_event_ids() {
    let store = InMemoryPaymentStore::default();
    let outbox = InMemoryOutboxStore::default();
    let provider = SandboxPaymentProvider::new("itest-secret");
    let payment = seed_payment(&store, 1_500).await;
    let body = webhook_payload("provider-event-1", &payment, "PaymentSucceeded", Utc::now());
    let signature = provider.sign_webhook(&body);

    let first = post_webhook(
        AppState::with_payment(store.clone(), outbox.clone(), provider.clone()),
        &signature,
        body.clone(),
    )
    .await;
    let second = post_webhook(
        AppState::with_payment(store.clone(), outbox.clone(), provider),
        &signature,
        body,
    )
    .await;

    assert_eq!(first.0, StatusCode::OK);
    assert_eq!(second.0, StatusCode::OK);
    let stored = store
        .find_payment(payment.id())
        .await
        .expect("lookup should work")
        .expect("payment should exist");
    assert_eq!(stored.status(), PaymentStatus::Succeeded);
    assert_eq!(
        outbox
            .claim_pending(10)
            .await
            .expect("outbox should be readable")
            .len(),
        1
    );
}

#[tokio::test]
async fn should_reject_an_invalid_webhook_signature() {
    let store = InMemoryPaymentStore::default();
    let outbox = InMemoryOutboxStore::default();
    let provider = SandboxPaymentProvider::new("itest-secret");
    let payment = seed_payment(&store, 1_500).await;
    let body = webhook_payload(
        "provider-event-bad-sig",
        &payment,
        "PaymentSucceeded",
        Utc::now(),
    );

    let (status, _) = post_webhook(
        AppState::with_payment(store.clone(), outbox.clone(), provider),
        "not-a-valid-signature",
        body,
    )
    .await;

    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let stored = store
        .find_payment(payment.id())
        .await
        .expect("lookup should work")
        .expect("payment should exist");
    assert_eq!(stored.status(), PaymentStatus::Created);
    assert!(
        outbox
            .claim_pending(10)
            .await
            .expect("outbox should be readable")
            .is_empty()
    );
}

#[tokio::test]
async fn should_emit_exactly_one_fulfillment_outbox_event_on_payment_success() {
    let store = InMemoryPaymentStore::default();
    let outbox = InMemoryOutboxStore::default();
    let provider = SandboxPaymentProvider::new("itest-secret");
    let payment = seed_payment(&store, 2_500).await;
    let body = webhook_payload(
        "provider-event-fulfill",
        &payment,
        "PaymentSucceeded",
        Utc::now(),
    );
    let signature = provider.sign_webhook(&body);

    let (status, _) = post_webhook(
        AppState::with_payment(store, outbox.clone(), provider),
        &signature,
        body,
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    let events = outbox
        .claim_pending(10)
        .await
        .expect("outbox should be readable");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type(), "fulfillment.requested");
    assert_eq!(events[0].aggregate_type(), "order");
    assert_eq!(events[0].aggregate_id(), payment.order_id().as_uuid());
}

#[tokio::test]
async fn should_not_let_a_refund_event_override_a_later_payment_fact() {
    let store = InMemoryPaymentStore::default();
    let outbox = InMemoryOutboxStore::default();
    let provider = SandboxPaymentProvider::new("itest-secret");
    let payment = seed_payment(&store, 1_500).await;
    let later = Utc::now();
    let earlier = later - Duration::seconds(90);

    let success_body = webhook_payload("provider-event-later", &payment, "PaymentSucceeded", later);
    let success_signature = provider.sign_webhook(&success_body);
    let (success_status, _) = post_webhook(
        AppState::with_payment(store.clone(), outbox.clone(), provider.clone()),
        &success_signature,
        success_body,
    )
    .await;
    assert_eq!(success_status, StatusCode::OK);

    let refund_body = webhook_payload(
        "provider-event-stale-refund",
        &payment,
        "RefundSucceeded",
        earlier,
    );
    let refund_signature = provider.sign_webhook(&refund_body);
    let (refund_status, _) = post_webhook(
        AppState::with_payment(store.clone(), outbox.clone(), provider),
        &refund_signature,
        refund_body,
    )
    .await;

    assert_eq!(refund_status, StatusCode::OK);
    let stored = store
        .find_payment(payment.id())
        .await
        .expect("lookup should work")
        .expect("payment should exist");
    assert_eq!(stored.status(), PaymentStatus::Succeeded);
    assert_eq!(
        outbox
            .claim_pending(10)
            .await
            .expect("outbox should be readable")
            .len(),
        1
    );
}

#[derive(Clone)]
struct TemporarilyUnavailablePublisher {
    attempts: Arc<AtomicUsize>,
}

#[async_trait]
impl EventPublisher for TemporarilyUnavailablePublisher {
    async fn publish(&self, _event: &OutboxEvent) -> Result<(), String> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        Err("temporarily unavailable".to_owned())
    }
}

#[tokio::test]
async fn should_keep_a_temporarily_unavailable_publish_retryable() {
    let store = InMemoryPaymentStore::default();
    let outbox = InMemoryOutboxStore::default();
    let provider = SandboxPaymentProvider::new("itest-secret");
    let payment = seed_payment(&store, 1_500).await;
    let body = webhook_payload(
        "provider-event-retryable",
        &payment,
        "PaymentSucceeded",
        Utc::now(),
    );
    let signature = provider.sign_webhook(&body);
    let (status, _) = post_webhook(
        AppState::with_payment(store, outbox.clone(), provider),
        &signature,
        body,
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let publisher = TemporarilyUnavailablePublisher {
        attempts: Arc::new(AtomicUsize::new(0)),
    };
    let worker = OutboxPublisher::new(outbox.clone(), publisher, 5);
    let published = worker
        .publish_pending(10)
        .await
        .expect("temporary publish failure should be recorded, not fatal");

    assert_eq!(published, 0);
    assert!(
        outbox
            .claim_pending(10)
            .await
            .expect("backoff should hide the row")
            .is_empty()
    );
    let events = outbox
        .claim_pending_at(Utc::now() + Duration::hours(1), 10)
        .await
        .expect("retryable events should be claimable after available_at");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status(), OutboxStatus::Pending);
    assert_eq!(events[0].attempts(), 1);
    assert_ne!(events[0].status(), OutboxStatus::DeadLetter);
}
