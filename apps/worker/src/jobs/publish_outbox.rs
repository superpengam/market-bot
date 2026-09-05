use async_trait::async_trait;
use chrono::{DateTime, Utc};
use market_bot_shared::{OutboxError, OutboxEvent, OutboxStore};

#[async_trait]
pub trait EventPublisher: Clone + Send + Sync + 'static {
    async fn publish(&self, event: &OutboxEvent) -> Result<(), String>;
}

#[derive(Clone)]
pub struct OutboxPublisher<S, P> {
    store: S,
    publisher: P,
    max_attempts: u32,
}

impl<S, P> OutboxPublisher<S, P>
where
    S: OutboxStore,
    P: EventPublisher,
{
    pub fn new(store: S, publisher: P, max_attempts: u32) -> Self {
        Self {
            store,
            publisher,
            max_attempts,
        }
    }

    pub async fn publish_pending(&self, batch_size: usize) -> Result<usize, OutboxError> {
        self.publish_pending_at(Utc::now(), batch_size).await
    }

    pub async fn publish_pending_at(
        &self,
        now: DateTime<Utc>,
        batch_size: usize,
    ) -> Result<usize, OutboxError> {
        let events = self.store.claim_pending_at(now, batch_size).await?;
        let mut published = 0;

        for event in events {
            match self.publisher.publish(&event).await {
                Ok(()) => {
                    self.store.mark_published(event.event_id()).await?;
                    published += 1;
                }
                Err(_reason) => {
                    // Why: a temporarily unavailable broker must leave the row
                    // pending (or failed-not-dead) so the next lease can retry
                    // after available_at.
                    self.store
                        .mark_failed_at(event.event_id(), self.max_attempts, now)
                        .await?;
                }
            }
        }

        Ok(published)
    }

    pub async fn run_once(&self, batch_size: usize) -> Result<usize, OutboxError> {
        self.publish_pending(batch_size).await
    }
}
