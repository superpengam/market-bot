use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use async_trait::async_trait;
use market_bot_shared::{OutboxStore, PaymentId};
use tokio::sync::Mutex;

use crate::{
    domain::{
        payment::{Payment, PaymentError, PaymentHandlingResult},
        refund::{Refund, RefundId},
    },
    ports::payment_repository::{PaymentRepository, PaymentUnitOfWork, WebhookApply},
};

#[derive(Default)]
struct PaymentState {
    payments: HashMap<PaymentId, Payment>,
    processed_event_ids: HashSet<String>,
    refunds: HashMap<RefundId, Refund>,
}

#[derive(Clone, Default)]
pub struct InMemoryPaymentStore {
    state: Arc<Mutex<PaymentState>>,
}

impl InMemoryPaymentStore {
    pub async fn save_payment(&self, payment: Payment) -> Result<(), PaymentError> {
        self.state
            .lock()
            .await
            .payments
            .insert(payment.id(), payment);
        Ok(())
    }

    pub async fn find_payment(
        &self,
        payment_id: PaymentId,
    ) -> Result<Option<Payment>, PaymentError> {
        Ok(self.state.lock().await.payments.get(&payment_id).cloned())
    }
}

#[async_trait]
impl PaymentRepository for InMemoryPaymentStore {
    async fn save_payment(&self, payment: Payment) -> Result<(), PaymentError> {
        InMemoryPaymentStore::save_payment(self, payment).await
    }

    async fn find_payment(&self, payment_id: PaymentId) -> Result<Option<Payment>, PaymentError> {
        InMemoryPaymentStore::find_payment(self, payment_id).await
    }

    async fn update_payment(&self, payment: Payment) -> Result<(), PaymentError> {
        let mut state = self.state.lock().await;
        if !state.payments.contains_key(&payment.id()) {
            return Err(PaymentError::PaymentNotFound);
        }

        state.payments.insert(payment.id(), payment);
        Ok(())
    }

    async fn record_event_id(&self, event_id: &str) -> Result<bool, PaymentError> {
        Ok(self
            .state
            .lock()
            .await
            .processed_event_ids
            .insert(event_id.to_owned()))
    }

    async fn forget_event_id(&self, event_id: &str) -> Result<(), PaymentError> {
        self.state.lock().await.processed_event_ids.remove(event_id);
        Ok(())
    }

    async fn save_refund(&self, refund: Refund) -> Result<(), PaymentError> {
        self.state.lock().await.refunds.insert(refund.id(), refund);
        Ok(())
    }

    async fn find_refund(&self, refund_id: RefundId) -> Result<Option<Refund>, PaymentError> {
        Ok(self.state.lock().await.refunds.get(&refund_id).cloned())
    }

    async fn find_refunds_by_payment(
        &self,
        payment_id: PaymentId,
    ) -> Result<Vec<Refund>, PaymentError> {
        Ok(self
            .state
            .lock()
            .await
            .refunds
            .values()
            .filter(|refund| refund.payment_id() == payment_id)
            .cloned()
            .collect())
    }

    async fn update_refund(&self, refund: Refund) -> Result<(), PaymentError> {
        let mut state = self.state.lock().await;
        if !state.refunds.contains_key(&refund.id()) {
            return Err(PaymentError::RefundNotFound);
        }

        state.refunds.insert(refund.id(), refund);
        Ok(())
    }
}

#[async_trait]
impl<O> PaymentUnitOfWork<O> for InMemoryPaymentStore
where
    O: OutboxStore,
{
    async fn commit_webhook(
        &self,
        outbox: &O,
        event_id: &str,
        payment_id: market_bot_shared::PaymentId,
        apply: Box<dyn FnOnce(Payment, Vec<Refund>) -> Result<WebhookApply, PaymentError> + Send>,
    ) -> Result<PaymentHandlingResult, PaymentError> {
        let mut state = self.state.lock().await;
        if state.processed_event_ids.contains(event_id) {
            return Ok(PaymentHandlingResult::Duplicate);
        }

        let payment = state
            .payments
            .get(&payment_id)
            .cloned()
            .ok_or(PaymentError::PaymentNotFound)?;
        let refunds: Vec<Refund> = state
            .refunds
            .values()
            .filter(|refund| refund.payment_id() == payment_id)
            .cloned()
            .collect();

        let outcome = apply(payment, refunds)?;
        if let Some(event) = outcome.outbox.clone() {
            outbox.append(event).await.map_err(PaymentError::Outbox)?;
        }

        state.processed_event_ids.insert(event_id.to_owned());
        if !outcome.stale {
            state.payments.insert(outcome.payment.id(), outcome.payment);
            for refund in outcome.refunds {
                state.refunds.insert(refund.id(), refund);
            }
        }

        Ok(if outcome.stale {
            PaymentHandlingResult::IgnoredStale
        } else {
            PaymentHandlingResult::Applied
        })
    }
}
