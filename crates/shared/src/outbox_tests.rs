use chrono::{Duration, Utc};
use serde_json::json;
use uuid::Uuid;

use super::{InMemoryOutboxStore, OutboxError, OutboxEvent, OutboxStatus, OutboxStore};

#[tokio::test]
async fn should_claim_and_mark_an_outbox_event_as_published() {
    let store = InMemoryOutboxStore::default();
    let event = OutboxEvent::new(
        "payment.succeeded",
        "payment",
        Uuid::new_v4(),
        json!({ "order_id": Uuid::new_v4() }),
    );
    let event_id = event.event_id();

    store.append(event).await.expect("event should append");
    let claimed = store.claim_pending(10).await.expect("event should claim");
    store
        .mark_published(event_id)
        .await
        .expect("event should publish");
    let remaining = store.claim_pending(10).await.expect("claim should work");

    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].status(), OutboxStatus::Pending);
    assert!(remaining.is_empty());
}

#[tokio::test]
async fn should_move_an_event_to_dead_letter_after_max_attempts() {
    let store = InMemoryOutboxStore::default();
    let event = OutboxEvent::new(
        "shipment.status_updated",
        "shipment",
        Uuid::new_v4(),
        json!({ "status": "exception" }),
    );
    let event_id = event.event_id();

    store.append(event).await.expect("event should append");
    store
        .mark_failed(event_id, 2)
        .await
        .expect("first failure should record");
    store
        .mark_failed(event_id, 2)
        .await
        .expect("second failure should record");

    let event = store
        .get(event_id)
        .await
        .expect("event lookup should work")
        .expect("event should exist");

    assert_eq!(event.status(), OutboxStatus::DeadLetter);
    assert_eq!(event.attempts(), 2);
}

#[tokio::test]
async fn should_reject_marking_an_unknown_event() {
    let store = InMemoryOutboxStore::default();

    assert_eq!(
        store.mark_published(Uuid::new_v4()).await,
        Err(OutboxError::EventNotFound)
    );
}

#[tokio::test]
async fn should_not_let_two_workers_claim_the_same_pending_event() {
    let store = InMemoryOutboxStore::default();
    let event = OutboxEvent::new(
        "fulfillment.requested",
        "order",
        Uuid::new_v4(),
        json!({ "lease": true }),
    );
    store.append(event).await.expect("event should append");

    let first = store
        .claim_pending(10)
        .await
        .expect("first worker should claim");
    let second = store
        .claim_pending(10)
        .await
        .expect("second worker should observe the lease");

    assert_eq!(first.len(), 1);
    assert_eq!(first[0].status(), OutboxStatus::Pending);
    assert!(
        second.is_empty(),
        "a claimed row must be hidden until its lease expires"
    );
}

#[tokio::test]
async fn should_hide_a_failed_event_until_its_backoff_elapses() {
    let store = InMemoryOutboxStore::default();
    let event = OutboxEvent::new(
        "fulfillment.requested",
        "order",
        Uuid::new_v4(),
        json!({ "backoff": true }),
    );
    let event_id = event.event_id();
    store.append(event).await.expect("event should append");

    store
        .mark_failed(event_id, 5)
        .await
        .expect("failure should record backoff");

    let stored = store
        .get(event_id)
        .await
        .expect("lookup should work")
        .expect("event should exist");
    assert_eq!(stored.status(), OutboxStatus::Pending);
    assert_eq!(stored.attempts(), 1);
    assert!(
        stored.available_at() > Utc::now(),
        "failure must push available_at into the future"
    );
    assert!(
        store
            .claim_pending(10)
            .await
            .expect("claim should work")
            .is_empty(),
        "backoff must hide the row from the next claim"
    );

    let later = Utc::now() + Duration::hours(1);
    let reclaimable = store
        .claim_pending_at(later, 10)
        .await
        .expect("elapsed backoff should make the row claimable");
    assert_eq!(reclaimable.len(), 1);
    assert_eq!(reclaimable[0].event_id(), event_id);
}
