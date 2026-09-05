use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use async_trait::async_trait;
use chrono::{Duration, Utc};
use market_bot_shared::{InMemoryOutboxStore, OutboxEvent, OutboxStatus, OutboxStore};
use serde_json::json;
use uuid::Uuid;

use super::jobs::publish_outbox::{EventPublisher, OutboxPublisher};

#[derive(Clone)]
struct FailingOncePublisher {
    attempts: Arc<AtomicUsize>,
}

#[async_trait]
impl EventPublisher for FailingOncePublisher {
    async fn publish(&self, _event: &OutboxEvent) -> Result<(), String> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        if attempt == 0 {
            Err("temporary queue failure".to_owned())
        } else {
            Ok(())
        }
    }
}

#[tokio::test]
async fn should_retry_an_outbox_event_after_a_temporary_publish_failure() {
    let store = InMemoryOutboxStore::default();
    let event = OutboxEvent::new(
        "product.published",
        "product",
        Uuid::new_v4(),
        json!({ "version": 1 }),
    );
    let event_id = event.event_id();
    store.append(event).await.expect("event should append");
    let publisher = FailingOncePublisher {
        attempts: Arc::new(AtomicUsize::new(0)),
    };
    let worker = OutboxPublisher::new(store.clone(), publisher, 3);

    let first = worker
        .publish_pending(10)
        .await
        .expect("temporary failure should be recorded");
    assert_eq!(first, 0);
    assert!(
        store.claim_pending(10).await.unwrap().is_empty(),
        "backoff must hide the row until available_at"
    );
    assert_eq!(
        store.get(event_id).await.unwrap().unwrap().status(),
        OutboxStatus::Pending
    );

    let published = worker
        .publish_pending_at(Utc::now() + Duration::hours(1), 10)
        .await
        .expect("retry should publish event");
    assert_eq!(published, 1);
    assert_eq!(
        store.get(event_id).await.unwrap().unwrap().status(),
        OutboxStatus::Published
    );
    assert_eq!(store.get(event_id).await.unwrap().unwrap().attempts(), 1);
}

#[tokio::test]
async fn should_keep_a_temporarily_unavailable_publish_retryable() {
    let store = InMemoryOutboxStore::default();
    let event = OutboxEvent::new(
        "fulfillment.requested",
        "order",
        Uuid::new_v4(),
        json!({ "version": 1 }),
    );
    let event_id = event.event_id();
    store.append(event).await.expect("event should append");
    let publisher = FailingOncePublisher {
        attempts: Arc::new(AtomicUsize::new(0)),
    };
    let worker = OutboxPublisher::new(store.clone(), publisher.clone(), 5);

    let published = worker
        .publish_pending(10)
        .await
        .expect("unavailable publish should not fail the batch");

    assert_eq!(published, 0);
    let stored = store
        .get(event_id)
        .await
        .expect("lookup should work")
        .expect("event should exist");
    assert_eq!(stored.status(), OutboxStatus::Pending);
    assert_eq!(stored.attempts(), 1);
    assert_ne!(stored.status(), OutboxStatus::DeadLetter);
}
