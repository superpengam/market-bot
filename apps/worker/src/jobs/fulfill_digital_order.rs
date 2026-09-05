use async_trait::async_trait;
use market_bot_fulfillment::{DeliveryReceipt, DigitalFulfillment};
use market_bot_shared::OrderId;

#[derive(Clone)]
pub struct FulfillDigitalOrderJob<D, S> {
    delivery: D,
    settlement: S,
}

impl<D, S> FulfillDigitalOrderJob<D, S> {
    pub fn new(delivery: D, settlement: S) -> Self {
        Self {
            delivery,
            settlement,
        }
    }
}

/// Settlement writes the worker must make after a digital receipt exists.
///
/// Why: the job must not embed payment rules. It only asks settlement to
/// record delivery and mark the order eligible.
#[async_trait]
pub trait SettlementGate: Clone + Send + Sync + 'static {
    type Error: std::error::Error + Send + Sync + 'static;

    async fn record_digital_delivered(&self, order_id: OrderId) -> Result<(), Self::Error>;

    async fn mark_eligible(&self, order_id: OrderId) -> Result<(), Self::Error>;
}

#[async_trait]
impl<Store, P> SettlementGate for market_bot_payment::SettlementService<Store, P>
where
    Store: market_bot_payment::SettlementStore,
    P: market_bot_payment::PaymentProvider,
{
    type Error = market_bot_payment::SettlementError;

    async fn record_digital_delivered(&self, order_id: OrderId) -> Result<(), Self::Error> {
        market_bot_payment::SettlementService::record_digital_delivered(self, order_id)
            .await
            .map(|_| ())
    }

    async fn mark_eligible(&self, order_id: OrderId) -> Result<(), Self::Error> {
        market_bot_payment::SettlementService::mark_eligible(self, order_id)
            .await
            .map(|_| ())
    }
}

#[derive(Debug)]
pub enum FulfillDigitalOrderError<DE, SE> {
    Delivery(DE),
    Settlement(SE),
}

impl<DE, SE> std::fmt::Display for FulfillDigitalOrderError<DE, SE>
where
    DE: std::fmt::Display,
    SE: std::fmt::Display,
{
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Delivery(error) => write!(formatter, "digital delivery failed: {error}"),
            Self::Settlement(error) => write!(formatter, "settlement update failed: {error}"),
        }
    }
}

impl<DE, SE> std::error::Error for FulfillDigitalOrderError<DE, SE>
where
    DE: std::error::Error + 'static,
    SE: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Delivery(error) => Some(error),
            Self::Settlement(error) => Some(error),
        }
    }
}

impl<D, S> FulfillDigitalOrderJob<D, S>
where
    D: DigitalFulfillment,
    S: SettlementGate,
{
    pub async fn run(
        &self,
        order_id: OrderId,
    ) -> Result<
        DeliveryReceipt,
        FulfillDigitalOrderError<market_bot_fulfillment::FulfillmentError, S::Error>,
    > {
        let receipt = self
            .delivery
            .fulfill(order_id)
            .await
            .map_err(FulfillDigitalOrderError::Delivery)?;
        self.settlement
            .record_digital_delivered(order_id)
            .await
            .map_err(FulfillDigitalOrderError::Settlement)?;
        self.settlement
            .mark_eligible(order_id)
            .await
            .map_err(FulfillDigitalOrderError::Settlement)?;
        Ok(receipt)
    }
}
