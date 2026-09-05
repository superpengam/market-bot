use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::Mutex;
use uuid::Uuid;

/// How long a successful `claim_pending` hides a row from other workers.
pub const OUTBOX_CLAIM_LEASE: Duration = Duration::seconds(30);

pub fn outbox_retry_backoff(attempts: u32) -> Duration {
    let shift = attempts.min(8);
    Duration::seconds(1_i64 << shift)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct OutboxEvent {
    event_id: Uuid,
    event_type: String,
    aggregate_type: String,
    aggregate_id: Uuid,
    payload: Value,
    status: OutboxStatus,
    attempts: u32,
    available_at: DateTime<Utc>,
}

impl OutboxEvent {
    pub fn new(
        event_type: impl Into<String>,
        aggregate_type: impl Into<String>,
        aggregate_id: Uuid,
        payload: Value,
    ) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            event_type: event_type.into(),
            aggregate_type: aggregate_type.into(),
            aggregate_id,
            payload,
            status: OutboxStatus::Pending,
            attempts: 0,
            available_at: Utc::now(),
        }
    }

    pub const fn event_id(&self) -> Uuid {
        self.event_id
    }

    pub fn event_type(&self) -> &str {
        &self.event_type
    }

    pub fn aggregate_type(&self) -> &str {
        &self.aggregate_type
    }

    pub const fn aggregate_id(&self) -> Uuid {
        self.aggregate_id
    }

    pub const fn payload(&self) -> &Value {
        &self.payload
    }

    pub const fn status(&self) -> OutboxStatus {
        self.status
    }

    pub const fn attempts(&self) -> u32 {
        self.attempts
    }

    pub const fn available_at(&self) -> DateTime<Utc> {
        self.available_at
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum OutboxStatus {
    Pending,
    Published,
    DeadLetter,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum OutboxError {
    #[error("outbox event already exists")]
    EventAlreadyExists,
    #[error("outbox event was not found")]
    EventNotFound,
    #[error("outbox maximum attempts must be greater than zero")]
    InvalidMaximumAttempts,
    #[error("outbox attempt count overflowed")]
    AttemptsOverflow,
}

#[async_trait]
pub trait OutboxStore: Clone + Send + Sync + 'static {
    async fn append(&self, event: OutboxEvent) -> Result<(), OutboxError>;
    async fn claim_pending(&self, limit: usize) -> Result<Vec<OutboxEvent>, OutboxError>;
    async fn claim_pending_at(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<OutboxEvent>, OutboxError>;
    async fn mark_published(&self, event_id: Uuid) -> Result<(), OutboxError>;
    async fn mark_failed(&self, event_id: Uuid, max_attempts: u32) -> Result<(), OutboxError>;
    async fn mark_failed_at(
        &self,
        event_id: Uuid,
        max_attempts: u32,
        now: DateTime<Utc>,
    ) -> Result<(), OutboxError>;
    async fn get(&self, event_id: Uuid) -> Result<Option<OutboxEvent>, OutboxError>;
}

#[derive(Clone, Default)]
pub struct InMemoryOutboxStore {
    events: Arc<Mutex<HashMap<Uuid, OutboxEvent>>>,
}

#[async_trait]
impl OutboxStore for InMemoryOutboxStore {
    async fn append(&self, event: OutboxEvent) -> Result<(), OutboxError> {
        let mut events = self.events.lock().await;
        if events.contains_key(&event.event_id()) {
            return Err(OutboxError::EventAlreadyExists);
        }

        events.insert(event.event_id(), event);
        Ok(())
    }

    async fn claim_pending(&self, limit: usize) -> Result<Vec<OutboxEvent>, OutboxError> {
        self.claim_pending_at(Utc::now(), limit).await
    }

    async fn claim_pending_at(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Result<Vec<OutboxEvent>, OutboxError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let mut events = self.events.lock().await;
        let lease_until = now + OUTBOX_CLAIM_LEASE;
        let ids: Vec<Uuid> = events
            .values()
            .filter(|event| event.status() == OutboxStatus::Pending && event.available_at() <= now)
            .map(OutboxEvent::event_id)
            .take(limit)
            .collect();

        let mut claimed = Vec::with_capacity(ids.len());
        for event_id in ids {
            if let Some(event) = events.get_mut(&event_id) {
                event.available_at = lease_until;
                claimed.push(event.clone());
            }
        }
        Ok(claimed)
    }

    async fn mark_published(&self, event_id: Uuid) -> Result<(), OutboxError> {
        let mut events = self.events.lock().await;
        let event = events
            .get_mut(&event_id)
            .ok_or(OutboxError::EventNotFound)?;
        event.status = OutboxStatus::Published;
        Ok(())
    }

    async fn mark_failed(&self, event_id: Uuid, max_attempts: u32) -> Result<(), OutboxError> {
        self.mark_failed_at(event_id, max_attempts, Utc::now())
            .await
    }

    async fn mark_failed_at(
        &self,
        event_id: Uuid,
        max_attempts: u32,
        now: DateTime<Utc>,
    ) -> Result<(), OutboxError> {
        if max_attempts == 0 {
            return Err(OutboxError::InvalidMaximumAttempts);
        }

        let mut events = self.events.lock().await;
        let event = events
            .get_mut(&event_id)
            .ok_or(OutboxError::EventNotFound)?;
        event.attempts = event
            .attempts
            .checked_add(1)
            .ok_or(OutboxError::AttemptsOverflow)?;
        if event.attempts >= max_attempts {
            event.status = OutboxStatus::DeadLetter;
        } else {
            event.available_at = now + outbox_retry_backoff(event.attempts);
        }
        Ok(())
    }

    async fn get(&self, event_id: Uuid) -> Result<Option<OutboxEvent>, OutboxError> {
        Ok(self.events.lock().await.get(&event_id).cloned())
    }
}
