use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use async_trait::async_trait;
use chrono::{DateTime, Duration, TimeZone, Utc};
use market_bot_shared::{
    CurrencyCode, InMemoryOutboxStore, Money, OrderId, OutboxError, OutboxEvent, OutboxStore,
    PaymentId,
};
use uuid::Uuid;

use super::{
    InMemoryPaymentStore, Payment, PaymentError, PaymentEvent, PaymentEventHandler,
    PaymentHandlingResult, PaymentRepository, PaymentStatus, PaymentUnitOfWork, Refund, RefundId,
    RefundStatus, VerifiedPaymentEvent, WebhookApply,
};

fn amount(minor: i64) -> Money {
    Money::new(
        minor,
        CurrencyCode::try_from("USD").expect("USD should be valid"),
    )
    .expect("amount should be valid")
}

fn at(year: i32, month: u32, day: u32, hour: u32, min: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, min, 0)
        .single()
        .expect("timestamp should be valid")
}

fn verified(event: PaymentEvent) -> VerifiedPaymentEvent {
    VerifiedPaymentEvent::from_verified(event)
}

#[tokio::test]
async fn should_process_a_webhook_once_and_emit_one_fulfillment_event() {
    let handler = PaymentEventHandler::new(
        InMemoryPaymentStore::default(),
        InMemoryOutboxStore::default(),
    );
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let event = PaymentEvent::succeeded("provider-event-1", payment.id(), payment.order_id());

    let first = handler
        .handle(verified(event.clone()))
        .await
        .expect("first webhook should work");
    let second = handler
        .handle(verified(event))
        .await
        .expect("duplicate webhook should be harmless");

    assert_eq!(first, PaymentHandlingResult::Applied);
    assert_eq!(second, PaymentHandlingResult::Duplicate);
}

#[tokio::test]
async fn should_write_a_fulfillment_request_to_the_shared_outbox() {
    let outbox = InMemoryOutboxStore::default();
    let handler = PaymentEventHandler::new(InMemoryPaymentStore::default(), outbox.clone());
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");

    handler
        .handle(verified(PaymentEvent::succeeded(
            "provider-event-outbox",
            payment.id(),
            payment.order_id(),
        )))
        .await
        .expect("payment event should be applied");

    let events = outbox
        .claim_pending(10)
        .await
        .expect("outbox should be readable");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_type(), "fulfillment.requested");
}

#[tokio::test]
async fn should_reject_a_refund_event_before_payment_succeeds() {
    let handler = PaymentEventHandler::new(
        InMemoryPaymentStore::default(),
        InMemoryOutboxStore::default(),
    );
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let event = PaymentEvent::refund_succeeded_with_amount(
        "provider-refund-1",
        payment.id(),
        payment.order_id(),
        Utc::now(),
        amount(1_500),
    );

    assert!(handler.handle(verified(event)).await.is_err());
}

#[tokio::test]
async fn should_ignore_a_stale_refund_event_after_a_later_payment_fact() {
    let store = InMemoryPaymentStore::default();
    let outbox = InMemoryOutboxStore::default();
    let handler = PaymentEventHandler::new(store.clone(), outbox.clone());
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let later = at(2026, 9, 3, 12, 0);
    let earlier = at(2026, 9, 3, 11, 58);

    handler
        .handle_at(
            verified(PaymentEvent::succeeded_at(
                "provider-event-later",
                payment.id(),
                payment.order_id(),
                later,
            )),
            later,
        )
        .await
        .expect("later payment fact should apply");

    let result = handler
        .handle_at(
            verified(PaymentEvent::refund_succeeded_at(
                "provider-event-stale-refund",
                payment.id(),
                payment.order_id(),
                earlier,
            )),
            later,
        )
        .await
        .expect("stale refund should be ignored, not fail");

    assert_eq!(result, PaymentHandlingResult::IgnoredStale);
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
async fn should_apply_a_newer_refund_event_after_payment_succeeded() {
    let store = InMemoryPaymentStore::default();
    let handler = PaymentEventHandler::new(store.clone(), InMemoryOutboxStore::default());
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let paid_at = at(2026, 9, 3, 12, 0);
    let refund_at = at(2026, 9, 3, 12, 2);

    handler
        .handle_at(
            verified(PaymentEvent::succeeded_at(
                "provider-event-paid",
                payment.id(),
                payment.order_id(),
                paid_at,
            )),
            paid_at,
        )
        .await
        .expect("payment should succeed");

    handler
        .handle_at(
            verified(PaymentEvent::refund_succeeded_with_amount(
                "provider-event-refund",
                payment.id(),
                payment.order_id(),
                refund_at,
                amount(1_500),
            )),
            refund_at,
        )
        .await
        .expect("newer refund should apply");

    let stored = store
        .find_payment(payment.id())
        .await
        .expect("lookup should work")
        .expect("payment should exist");
    assert_eq!(stored.status(), PaymentStatus::Refunded);
}

#[tokio::test]
async fn should_reject_an_event_outside_the_time_window() {
    let handler = PaymentEventHandler::new(
        InMemoryPaymentStore::default(),
        InMemoryOutboxStore::default(),
    );
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let occurred_at = at(2026, 9, 3, 12, 0);
    let now = occurred_at + Duration::seconds(301);
    let event = PaymentEvent::succeeded_at(
        "provider-event-expired",
        payment.id(),
        payment.order_id(),
        occurred_at,
    );

    let error = handler
        .handle_at(verified(event), now)
        .await
        .expect_err("expired event should be rejected");

    assert_eq!(error, super::PaymentError::EventOutsideTimeWindow);
}

#[tokio::test]
async fn should_process_a_refund_webhook_once() {
    let handler = PaymentEventHandler::new(
        InMemoryPaymentStore::default(),
        InMemoryOutboxStore::default(),
    );
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let paid_at = at(2026, 9, 3, 12, 0);
    handler
        .handle_at(
            verified(PaymentEvent::succeeded_at(
                "provider-event-paid-once",
                payment.id(),
                payment.order_id(),
                paid_at,
            )),
            paid_at,
        )
        .await
        .expect("payment should succeed");

    let refund = PaymentEvent::refund_succeeded_with_amount(
        "provider-refund-once",
        payment.id(),
        payment.order_id(),
        paid_at + Duration::seconds(30),
        amount(1_500),
    );
    let first = handler
        .handle_at(verified(refund.clone()), paid_at + Duration::seconds(30))
        .await
        .expect("refund should apply");
    let second = handler
        .handle_at(verified(refund), paid_at + Duration::seconds(30))
        .await
        .expect("duplicate refund webhook should be ignored");

    assert_eq!(first, PaymentHandlingResult::Applied);
    assert_eq!(second, PaymentHandlingResult::Duplicate);
}

#[tokio::test]
async fn should_keep_payment_open_after_a_partial_refund() {
    let store = InMemoryPaymentStore::default();
    let handler = PaymentEventHandler::new(store.clone(), InMemoryOutboxStore::default());
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let paid_at = at(2026, 9, 3, 12, 0);
    handler
        .handle_at(
            verified(PaymentEvent::succeeded_at(
                "provider-event-paid-partial",
                payment.id(),
                payment.order_id(),
                paid_at,
            )),
            paid_at,
        )
        .await
        .expect("payment should succeed");

    let mut refund_event = PaymentEvent::refund_succeeded_at(
        "provider-event-partial-refund",
        payment.id(),
        payment.order_id(),
        paid_at + Duration::seconds(30),
    );
    refund_event.amount = Some(amount(500));
    handler
        .handle_at(verified(refund_event), paid_at + Duration::seconds(30))
        .await
        .expect("partial refund should apply");

    let stored = store
        .find_payment(payment.id())
        .await
        .expect("lookup should work")
        .expect("payment should exist");
    assert_eq!(stored.status(), PaymentStatus::RefundProcessing);
    assert_eq!(stored.refunded_amount().minor(), 500);
}

#[tokio::test]
async fn should_mark_only_the_matching_open_refund() {
    let store = InMemoryPaymentStore::default();
    let handler = PaymentEventHandler::new(store.clone(), InMemoryOutboxStore::default());
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let paid_at = at(2026, 9, 3, 12, 0);
    handler
        .handle_at(
            verified(PaymentEvent::succeeded_at(
                "provider-event-paid-match",
                payment.id(),
                payment.order_id(),
                paid_at,
            )),
            paid_at,
        )
        .await
        .expect("payment should succeed");

    let mut first = Refund::new(payment.id(), payment.order_id(), amount(500), "partial one");
    first.mark_processing("refund-a");
    let mut second = Refund::new(payment.id(), payment.order_id(), amount(700), "partial two");
    second.mark_processing("refund-b");
    store
        .save_refund(first.clone())
        .await
        .expect("first refund should persist");
    store
        .save_refund(second.clone())
        .await
        .expect("second refund should persist");

    let mut refund_event = PaymentEvent::refund_succeeded_at(
        "provider-event-match-refund",
        payment.id(),
        payment.order_id(),
        paid_at + Duration::seconds(30),
    );
    refund_event.amount = Some(amount(500));
    handler
        .handle_at(verified(refund_event), paid_at + Duration::seconds(30))
        .await
        .expect("matching refund should apply");

    let refunds = store
        .find_refunds_by_payment(payment.id())
        .await
        .expect("refunds should load");
    let first_status = refunds
        .iter()
        .find(|refund| refund.id() == first.id())
        .expect("first refund should exist")
        .status();
    let second_status = refunds
        .iter()
        .find(|refund| refund.id() == second.id())
        .expect("second refund should exist")
        .status();
    assert_eq!(first_status, RefundStatus::Succeeded);
    assert_eq!(second_status, RefundStatus::Processing);
}

#[tokio::test]
async fn should_require_an_amount_on_refund_succeeded() {
    let handler = PaymentEventHandler::new(
        InMemoryPaymentStore::default(),
        InMemoryOutboxStore::default(),
    );
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let paid_at = at(2026, 9, 3, 12, 0);
    handler
        .handle_at(
            verified(PaymentEvent::succeeded_at(
                "provider-event-paid-require-amount",
                payment.id(),
                payment.order_id(),
                paid_at,
            )),
            paid_at,
        )
        .await
        .expect("payment should succeed");

    let error = handler
        .handle_at(
            verified(PaymentEvent::refund_succeeded_at(
                "provider-event-refund-no-amount",
                payment.id(),
                payment.order_id(),
                paid_at + Duration::seconds(30),
            )),
            paid_at + Duration::seconds(30),
        )
        .await
        .expect_err("refund succeeded must carry an amount");

    assert_eq!(error, PaymentError::InvalidRefundAmount);
}

#[tokio::test]
async fn should_cap_a_refund_callback_at_the_remaining_balance() {
    let store = InMemoryPaymentStore::default();
    let handler = PaymentEventHandler::new(store.clone(), InMemoryOutboxStore::default());
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let paid_at = at(2026, 9, 3, 12, 0);
    handler
        .handle_at(
            verified(PaymentEvent::succeeded_at(
                "provider-event-paid-cap",
                payment.id(),
                payment.order_id(),
                paid_at,
            )),
            paid_at,
        )
        .await
        .expect("payment should succeed");

    let mut refund_event = PaymentEvent::refund_succeeded_at(
        "provider-event-over-refund",
        payment.id(),
        payment.order_id(),
        paid_at + Duration::seconds(30),
    );
    refund_event.amount = Some(amount(2_000));
    handler
        .handle_at(verified(refund_event), paid_at + Duration::seconds(30))
        .await
        .expect("over-refund should cap");

    let stored = store
        .find_payment(payment.id())
        .await
        .expect("lookup should work")
        .expect("payment should exist");
    assert_eq!(stored.status(), PaymentStatus::Refunded);
    assert_eq!(stored.refunded_amount().minor(), 1_500);
}

#[derive(Clone, Default)]
struct NonRevertingStore {
    inner: InMemoryPaymentStore,
}

#[async_trait]
impl PaymentRepository for NonRevertingStore {
    async fn save_payment(&self, payment: Payment) -> Result<(), PaymentError> {
        self.inner.save_payment(payment).await
    }

    async fn find_payment(&self, payment_id: PaymentId) -> Result<Option<Payment>, PaymentError> {
        self.inner.find_payment(payment_id).await
    }

    async fn update_payment(&self, payment: Payment) -> Result<(), PaymentError> {
        if let Some(current) = self.inner.find_payment(payment.id()).await?
            && current.status() == PaymentStatus::Succeeded
            && payment.status() == PaymentStatus::Created
        {
            return Ok(());
        }
        self.inner.update_payment(payment).await
    }

    async fn record_event_id(&self, event_id: &str) -> Result<bool, PaymentError> {
        self.inner.record_event_id(event_id).await
    }

    async fn forget_event_id(&self, event_id: &str) -> Result<(), PaymentError> {
        self.inner.forget_event_id(event_id).await
    }

    async fn save_refund(&self, refund: Refund) -> Result<(), PaymentError> {
        self.inner.save_refund(refund).await
    }

    async fn find_refund(&self, refund_id: RefundId) -> Result<Option<Refund>, PaymentError> {
        self.inner.find_refund(refund_id).await
    }

    async fn find_refunds_by_payment(
        &self,
        payment_id: PaymentId,
    ) -> Result<Vec<Refund>, PaymentError> {
        self.inner.find_refunds_by_payment(payment_id).await
    }

    async fn update_refund(&self, refund: Refund) -> Result<(), PaymentError> {
        self.inner.update_refund(refund).await
    }
}

#[async_trait]
impl<O> PaymentUnitOfWork<O> for NonRevertingStore
where
    O: OutboxStore,
{
    async fn commit_webhook(
        &self,
        outbox: &O,
        event_id: &str,
        payment_id: PaymentId,
        apply: Box<dyn FnOnce(Payment, Vec<Refund>) -> Result<WebhookApply, PaymentError> + Send>,
    ) -> Result<PaymentHandlingResult, PaymentError> {
        self.inner
            .commit_webhook(outbox, event_id, payment_id, apply)
            .await
    }
}

#[derive(Clone, Default)]
struct YieldingStore {
    inner: InMemoryPaymentStore,
}

#[async_trait]
impl PaymentRepository for YieldingStore {
    async fn save_payment(&self, payment: Payment) -> Result<(), PaymentError> {
        self.inner.save_payment(payment).await
    }

    async fn find_payment(&self, payment_id: PaymentId) -> Result<Option<Payment>, PaymentError> {
        let payment = self.inner.find_payment(payment_id).await?;
        tokio::task::yield_now().await;
        Ok(payment)
    }

    async fn update_payment(&self, payment: Payment) -> Result<(), PaymentError> {
        self.inner.update_payment(payment).await
    }

    async fn record_event_id(&self, event_id: &str) -> Result<bool, PaymentError> {
        self.inner.record_event_id(event_id).await
    }

    async fn forget_event_id(&self, event_id: &str) -> Result<(), PaymentError> {
        self.inner.forget_event_id(event_id).await
    }

    async fn save_refund(&self, refund: Refund) -> Result<(), PaymentError> {
        self.inner.save_refund(refund).await
    }

    async fn find_refund(&self, refund_id: RefundId) -> Result<Option<Refund>, PaymentError> {
        self.inner.find_refund(refund_id).await
    }

    async fn find_refunds_by_payment(
        &self,
        payment_id: PaymentId,
    ) -> Result<Vec<Refund>, PaymentError> {
        self.inner.find_refunds_by_payment(payment_id).await
    }

    async fn update_refund(&self, refund: Refund) -> Result<(), PaymentError> {
        self.inner.update_refund(refund).await
    }
}

#[async_trait]
impl<O> PaymentUnitOfWork<O> for YieldingStore
where
    O: OutboxStore,
{
    async fn commit_webhook(
        &self,
        outbox: &O,
        event_id: &str,
        payment_id: PaymentId,
        apply: Box<dyn FnOnce(Payment, Vec<Refund>) -> Result<WebhookApply, PaymentError> + Send>,
    ) -> Result<PaymentHandlingResult, PaymentError> {
        self.inner
            .commit_webhook(outbox, event_id, payment_id, apply)
            .await
    }
}

#[derive(Clone)]
struct FailOnceOutbox {
    inner: InMemoryOutboxStore,
    failures_remaining: Arc<AtomicU32>,
}

impl FailOnceOutbox {
    fn once() -> Self {
        Self {
            inner: InMemoryOutboxStore::default(),
            failures_remaining: Arc::new(AtomicU32::new(1)),
        }
    }
}

#[async_trait]
impl OutboxStore for FailOnceOutbox {
    async fn append(&self, event: OutboxEvent) -> Result<(), OutboxError> {
        if self.failures_remaining.fetch_sub(1, Ordering::SeqCst) > 0 {
            return Err(OutboxError::EventAlreadyExists);
        }
        self.inner.append(event).await
    }

    async fn claim_pending(&self, limit: usize) -> Result<Vec<OutboxEvent>, OutboxError> {
        self.inner.claim_pending(limit).await
    }

    async fn claim_pending_at(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<OutboxEvent>, OutboxError> {
        self.inner.claim_pending_at(now, limit).await
    }

    async fn mark_published(&self, event_id: Uuid) -> Result<(), OutboxError> {
        self.inner.mark_published(event_id).await
    }

    async fn mark_failed(&self, event_id: Uuid, max_attempts: u32) -> Result<(), OutboxError> {
        self.inner.mark_failed(event_id, max_attempts).await
    }

    async fn mark_failed_at(
        &self,
        event_id: Uuid,
        max_attempts: u32,
        now: DateTime<Utc>,
    ) -> Result<(), OutboxError> {
        self.inner.mark_failed_at(event_id, max_attempts, now).await
    }

    async fn get(&self, event_id: Uuid) -> Result<Option<OutboxEvent>, OutboxError> {
        self.inner.get(event_id).await
    }
}

#[tokio::test]
async fn should_emit_fulfillment_on_retry_when_outbox_append_fails() {
    let store = NonRevertingStore::default();
    let outbox = FailOnceOutbox::once();
    let handler = PaymentEventHandler::new(store.clone(), outbox.clone());
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");
    let event = verified(PaymentEvent::succeeded(
        "provider-event-outbox-retry",
        payment.id(),
        payment.order_id(),
    ));

    handler
        .handle(event.clone())
        .await
        .expect_err("first append should fail");
    handler
        .handle(event)
        .await
        .expect("retry must still be able to apply the success fact");

    let events = outbox
        .claim_pending(10)
        .await
        .expect("outbox should be readable");
    assert_eq!(
        events.len(),
        1,
        "a failed outbox commit must not consume the fulfillment"
    );
    assert_eq!(events[0].event_type(), "fulfillment.requested");
}

#[tokio::test]
async fn should_emit_only_one_fulfillment_for_two_success_event_ids() {
    let store = YieldingStore::default();
    let outbox = InMemoryOutboxStore::default();
    let handler = PaymentEventHandler::new(store.clone(), outbox.clone());
    let payment = handler
        .create_payment(OrderId::new(), amount(1_500))
        .await
        .expect("payment should be created");

    let first = handler.handle(verified(PaymentEvent::succeeded(
        "provider-event-success-a",
        payment.id(),
        payment.order_id(),
    )));
    let second = handler.handle(verified(PaymentEvent::succeeded(
        "provider-event-success-b",
        payment.id(),
        payment.order_id(),
    )));
    let (first, second) = tokio::join!(first, second);
    first.expect("first success event should apply");
    second.expect("second success event should apply");

    let events = outbox
        .claim_pending(10)
        .await
        .expect("outbox should be readable");
    assert_eq!(
        events.len(),
        1,
        "two success event IDs must not emit two fulfillment rows"
    );
}
