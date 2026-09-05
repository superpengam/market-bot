use chrono::{DateTime, Utc};
use market_bot_shared::{Money, OrderId, SellerId, SettlementId};
use serde::{Deserialize, Serialize};

/// Seller payout state for a single order.
///
/// Invariant: buyer funds stay with the payment provider. This entity stores
/// provider references and eligibility only — never a platform wallet balance.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Settlement {
    id: SettlementId,
    order_id: OrderId,
    seller_id: SellerId,
    amount: Money,
    status: SettlementStatus,
    auto_confirm_at: Option<DateTime<Utc>>,
    digital_delivered_at: Option<DateTime<Utc>>,
    eligible_at: Option<DateTime<Utc>>,
    block_reason: Option<SettlementBlockReason>,
    provider_settlement_id: Option<String>,
}

impl Settlement {
    pub fn pending(
        order_id: OrderId,
        seller_id: SellerId,
        amount: Money,
        auto_confirm_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: SettlementId::new(),
            order_id,
            seller_id,
            amount,
            status: SettlementStatus::Pending,
            auto_confirm_at,
            digital_delivered_at: None,
            eligible_at: None,
            block_reason: None,
            provider_settlement_id: None,
        }
    }

    pub const fn id(&self) -> SettlementId {
        self.id
    }

    pub const fn order_id(&self) -> OrderId {
        self.order_id
    }

    pub const fn seller_id(&self) -> SellerId {
        self.seller_id
    }

    pub const fn amount(&self) -> &Money {
        &self.amount
    }

    pub const fn status(&self) -> SettlementStatus {
        self.status
    }

    pub const fn auto_confirm_at(&self) -> Option<DateTime<Utc>> {
        self.auto_confirm_at
    }

    pub const fn digital_delivered_at(&self) -> Option<DateTime<Utc>> {
        self.digital_delivered_at
    }

    pub const fn eligible_at(&self) -> Option<DateTime<Utc>> {
        self.eligible_at
    }

    pub const fn block_reason(&self) -> Option<SettlementBlockReason> {
        self.block_reason
    }

    pub fn provider_settlement_id(&self) -> Option<&str> {
        self.provider_settlement_id.as_deref()
    }

    /// Invariant: the platform never hosts seller funds. Buyer money remains
    /// at the payment provider until `release` asks that provider to pay out.
    pub const fn wallet_balance(&self) -> Option<Money> {
        None
    }

    pub const fn is_blocked(&self) -> bool {
        self.block_reason.is_some()
    }

    /// Records that digital fulfillment succeeded. Does not move money.
    pub fn record_digital_delivered(&mut self, at: DateTime<Utc>) -> Result<(), SettlementError> {
        if self.status == SettlementStatus::Released {
            return Err(SettlementError::AlreadyReleased);
        }
        if self.digital_delivered_at.is_none() {
            self.digital_delivered_at = Some(at);
        }
        Ok(())
    }

    /// Why: a refund or open dispute means the buyer may still be owed the
    /// captured funds. Those funds must not be paid out to the seller.
    ///
    /// Safety: after this returns, `mark_eligible` and `release` must fail
    /// without calling the payment provider.
    pub fn record_block(&mut self, reason: SettlementBlockReason) -> Result<(), SettlementError> {
        if self.status == SettlementStatus::Released {
            return Err(SettlementError::AlreadyReleased);
        }
        self.block_reason = Some(reason);
        self.status = SettlementStatus::Blocked;
        Ok(())
    }

    pub fn mark_eligible(&mut self, at: DateTime<Utc>) -> Result<(), SettlementError> {
        if let Some(reason) = self.block_reason {
            return Err(SettlementError::Blocked { reason });
        }
        if self.status == SettlementStatus::Released {
            return Err(SettlementError::AlreadyReleased);
        }
        if self.status == SettlementStatus::Eligible {
            return Ok(());
        }
        if self.digital_delivered_at.is_none() {
            return Err(SettlementError::NotEligible);
        }
        self.status = SettlementStatus::Eligible;
        self.eligible_at = Some(self.eligible_at.unwrap_or(at));
        Ok(())
    }

    pub fn mark_released(
        &mut self,
        provider_settlement_id: impl Into<String>,
    ) -> Result<(), SettlementError> {
        self.ensure_releasable()?;
        self.status = SettlementStatus::Released;
        self.provider_settlement_id = Some(provider_settlement_id.into());
        Ok(())
    }

    pub fn ensure_releasable(&self) -> Result<(), SettlementError> {
        if let Some(reason) = self.block_reason {
            return Err(SettlementError::Blocked { reason });
        }
        if self.status == SettlementStatus::Released {
            return Err(SettlementError::AlreadyReleased);
        }
        if self.status != SettlementStatus::Eligible {
            return Err(SettlementError::NotEligible);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SettlementStatus {
    Pending,
    Eligible,
    Released,
    Blocked,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SettlementBlockReason {
    Refund,
    Dispute,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum SettlementError {
    #[error("settlement was not found")]
    NotFound,
    #[error("a settlement already exists for this order")]
    AlreadyExists,
    #[error("settlement is blocked ({reason:?})")]
    Blocked { reason: SettlementBlockReason },
    #[error("settlement is not eligible for release")]
    NotEligible,
    #[error("settlement has already been released")]
    AlreadyReleased,
    #[error("settlement provider is temporarily unavailable")]
    ProviderTemporarilyUnavailable,
    #[error("settlement provider request failed")]
    ProviderFailed,
}
