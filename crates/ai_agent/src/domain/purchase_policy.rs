use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::AiError;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PurchasePolicy {
    pub allowed_categories: BTreeSet<String>,
    pub max_order_minor: i64,
    pub max_daily_minor: i64,
    pub max_monthly_minor: i64,
    pub max_shipping_minor: i64,
    pub allowed_seller_score: i32,
    pub require_price_reconfirmation: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PurchaseEvaluation {
    pub category_id: String,
    pub seller_score: i32,
    pub order_total_minor: i64,
    pub shipping_minor: i64,
    pub daily_spent_minor: i64,
    pub monthly_spent_minor: i64,
    pub quoted_unit_price_minor: i64,
    pub catalog_unit_price_minor: i64,
    pub quoted_shipping_minor: i64,
    pub catalog_shipping_minor: i64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecision {
    Allowed,
    RequiresUserConfirmation { reason: PolicyReason },
    Blocked { reason: PolicyReason },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyReason {
    CategoryNotAllowed,
    SellerScoreTooLow,
    MaxOrderExceeded,
    MaxDailyExceeded,
    MaxMonthlyExceeded,
    MaxShippingExceeded,
    PriceChanged,
    ShippingChanged,
}

impl fmt::Display for PolicyReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::CategoryNotAllowed => "category_not_allowed",
            Self::SellerScoreTooLow => "seller_score_too_low",
            Self::MaxOrderExceeded => "max_order_exceeded",
            Self::MaxDailyExceeded => "max_daily_exceeded",
            Self::MaxMonthlyExceeded => "max_monthly_exceeded",
            Self::MaxShippingExceeded => "max_shipping_exceeded",
            Self::PriceChanged => "price_changed",
            Self::ShippingChanged => "shipping_changed",
        })
    }
}

impl PurchasePolicy {
    pub fn new(
        allowed_categories: impl IntoIterator<Item = impl Into<String>>,
        max_order_minor: i64,
        max_daily_minor: i64,
        max_monthly_minor: i64,
        max_shipping_minor: i64,
        allowed_seller_score: i32,
        require_price_reconfirmation: bool,
    ) -> Result<Self, AiError> {
        if [
            max_order_minor,
            max_daily_minor,
            max_monthly_minor,
            max_shipping_minor,
        ]
        .iter()
        .any(|amount| *amount < 0)
        {
            return Err(AiError::InvalidPolicyAmount);
        }

        Ok(Self {
            allowed_categories: allowed_categories.into_iter().map(Into::into).collect(),
            max_order_minor,
            max_daily_minor,
            max_monthly_minor,
            max_shipping_minor,
            allowed_seller_score,
            require_price_reconfirmation,
        })
    }

    /// Why: policy outcomes are computed here so callers cannot invent Allowed.
    /// Safety: search quotes are never used as the order-total fact.
    #[must_use]
    pub fn evaluate(&self, evaluation: &PurchaseEvaluation) -> PolicyDecision {
        if !self.allowed_categories.contains(&evaluation.category_id) {
            return PolicyDecision::Blocked {
                reason: PolicyReason::CategoryNotAllowed,
            };
        }
        if evaluation.seller_score < self.allowed_seller_score {
            return PolicyDecision::Blocked {
                reason: PolicyReason::SellerScoreTooLow,
            };
        }
        if evaluation.order_total_minor > self.max_order_minor {
            return PolicyDecision::Blocked {
                reason: PolicyReason::MaxOrderExceeded,
            };
        }
        if exceeds_cap(
            evaluation.daily_spent_minor,
            evaluation.order_total_minor,
            self.max_daily_minor,
        ) {
            return PolicyDecision::Blocked {
                reason: PolicyReason::MaxDailyExceeded,
            };
        }
        if exceeds_cap(
            evaluation.monthly_spent_minor,
            evaluation.order_total_minor,
            self.max_monthly_minor,
        ) {
            return PolicyDecision::Blocked {
                reason: PolicyReason::MaxMonthlyExceeded,
            };
        }
        if evaluation.shipping_minor > self.max_shipping_minor {
            return PolicyDecision::RequiresUserConfirmation {
                reason: PolicyReason::MaxShippingExceeded,
            };
        }
        if self.require_price_reconfirmation
            && evaluation.quoted_unit_price_minor != evaluation.catalog_unit_price_minor
        {
            return PolicyDecision::RequiresUserConfirmation {
                reason: PolicyReason::PriceChanged,
            };
        }
        if self.require_price_reconfirmation
            && evaluation.quoted_shipping_minor != evaluation.catalog_shipping_minor
        {
            return PolicyDecision::RequiresUserConfirmation {
                reason: PolicyReason::ShippingChanged,
            };
        }

        PolicyDecision::Allowed
    }
}

impl PolicyDecision {
    pub const fn into_result(self) -> Result<(), AiError> {
        match self {
            Self::Allowed => Ok(()),
            Self::RequiresUserConfirmation { reason } => {
                Err(AiError::RequiresUserConfirmation { reason })
            }
            Self::Blocked { reason } => Err(AiError::PolicyBlocked { reason }),
        }
    }
}

fn exceeds_cap(spent_minor: i64, additional_minor: i64, cap_minor: i64) -> bool {
    spent_minor
        .checked_add(additional_minor)
        .map(|total| total > cap_minor)
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::{PolicyDecision, PolicyReason, PurchaseEvaluation, PurchasePolicy};

    fn policy() -> PurchasePolicy {
        PurchasePolicy::new(["electronics"], 10_000, 20_000, 50_000, 1_500, 80, true)
            .expect("policy should be valid")
    }

    fn evaluation() -> PurchaseEvaluation {
        PurchaseEvaluation {
            category_id: "electronics".to_owned(),
            seller_score: 90,
            order_total_minor: 8_000,
            shipping_minor: 500,
            daily_spent_minor: 0,
            monthly_spent_minor: 0,
            quoted_unit_price_minor: 7_500,
            catalog_unit_price_minor: 7_500,
            quoted_shipping_minor: 500,
            catalog_shipping_minor: 500,
        }
    }

    #[test]
    fn should_allow_a_purchase_inside_every_limit() {
        assert_eq!(policy().evaluate(&evaluation()), PolicyDecision::Allowed);
    }

    #[test]
    fn should_block_when_order_daily_or_monthly_caps_are_exceeded() {
        let policy = policy();
        let mut over_order = evaluation();
        over_order.order_total_minor = 10_001;
        assert!(matches!(
            policy.evaluate(&over_order),
            PolicyDecision::Blocked {
                reason: PolicyReason::MaxOrderExceeded
            }
        ));

        let mut over_daily = evaluation();
        over_daily.daily_spent_minor = 15_000;
        assert!(matches!(
            policy.evaluate(&over_daily),
            PolicyDecision::Blocked {
                reason: PolicyReason::MaxDailyExceeded
            }
        ));

        let mut over_monthly = evaluation();
        over_monthly.monthly_spent_minor = 49_000;
        over_monthly.order_total_minor = 2_000;
        assert!(matches!(
            policy.evaluate(&over_monthly),
            PolicyDecision::Blocked {
                reason: PolicyReason::MaxMonthlyExceeded
            }
        ));
    }

    #[test]
    fn should_require_confirmation_when_price_or_shipping_exceeds_policy() {
        let policy = policy();
        let mut price_changed = evaluation();
        price_changed.catalog_unit_price_minor = 8_200;
        assert!(matches!(
            policy.evaluate(&price_changed),
            PolicyDecision::RequiresUserConfirmation {
                reason: PolicyReason::PriceChanged
            }
        ));

        let mut shipping_over = evaluation();
        shipping_over.shipping_minor = 1_501;
        shipping_over.catalog_shipping_minor = 1_501;
        shipping_over.quoted_shipping_minor = 1_501;
        assert!(matches!(
            policy.evaluate(&shipping_over),
            PolicyDecision::RequiresUserConfirmation {
                reason: PolicyReason::MaxShippingExceeded
            }
        ));
    }
}
