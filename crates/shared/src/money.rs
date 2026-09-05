use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct CurrencyCode(String);

impl CurrencyCode {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for CurrencyCode {
    type Error = CurrencyCodeError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value.len() != 3 || !value.bytes().all(|byte| byte.is_ascii_uppercase()) {
            return Err(CurrencyCodeError::InvalidFormat);
        }

        Ok(Self(value.to_owned()))
    }
}

impl TryFrom<String> for CurrencyCode {
    type Error = CurrencyCodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl AsRef<str> for CurrencyCode {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl std::fmt::Display for CurrencyCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum CurrencyCodeError {
    #[error("currency code must contain three uppercase ASCII letters")]
    InvalidFormat,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Money {
    minor: i64,
    currency: CurrencyCode,
}

impl Money {
    pub fn new(minor: i64, currency: CurrencyCode) -> Result<Self, MoneyError> {
        if minor < 0 {
            return Err(MoneyError::NegativeAmount);
        }

        Ok(Self { minor, currency })
    }

    pub fn minor(&self) -> i64 {
        self.minor
    }

    pub fn currency(&self) -> &CurrencyCode {
        &self.currency
    }

    pub fn checked_add(self, other: Self) -> Result<Self, MoneyError> {
        self.ensure_same_currency(&other)?;
        let minor = self
            .minor
            .checked_add(other.minor)
            .ok_or(MoneyError::Overflow)?;

        Ok(Self {
            minor,
            currency: self.currency,
        })
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, MoneyError> {
        self.ensure_same_currency(&other)?;
        let minor = self
            .minor
            .checked_sub(other.minor)
            .ok_or(MoneyError::NegativeAmount)?;

        Ok(Self {
            minor,
            currency: self.currency,
        })
    }

    pub fn checked_mul(self, quantity: u64) -> Result<Self, MoneyError> {
        let quantity = i64::try_from(quantity).map_err(|_| MoneyError::Overflow)?;
        let minor = self
            .minor
            .checked_mul(quantity)
            .ok_or(MoneyError::Overflow)?;

        Ok(Self {
            minor,
            currency: self.currency,
        })
    }

    fn ensure_same_currency(&self, other: &Self) -> Result<(), MoneyError> {
        if self.currency != other.currency {
            return Err(MoneyError::CurrencyMismatch);
        }

        Ok(())
    }
}

impl TryFrom<(i64, CurrencyCode)> for Money {
    type Error = MoneyError;

    fn try_from((minor, currency): (i64, CurrencyCode)) -> Result<Self, Self::Error> {
        Self::new(minor, currency)
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum MoneyError {
    #[error("money amount cannot be negative")]
    NegativeAmount,
    #[error("money values must use the same currency")]
    CurrencyMismatch,
    #[error("money arithmetic overflowed")]
    Overflow,
}
