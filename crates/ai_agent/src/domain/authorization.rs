use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use market_bot_shared::{AiAuthorizationId, UserId};
use serde::{Deserialize, Serialize};

use crate::AiError;

/// API scope strings used by AI clients. `order:create` never implies auto-purchase.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum AiScope {
    CatalogRead,
    CartRead,
    CartWrite,
    CheckoutPreview,
    OrderCreate,
    OrderRead,
    AutoPurchase,
}

impl AiScope {
    pub const fn as_api_str(self) -> &'static str {
        match self {
            Self::CatalogRead => "catalog:read",
            Self::CartRead => "cart:read",
            Self::CartWrite => "cart:write",
            Self::CheckoutPreview => "checkout:preview",
            Self::OrderCreate => "order:create",
            Self::OrderRead => "order:read",
            Self::AutoPurchase => "order:auto_purchase",
        }
    }

    pub fn parse(value: &str) -> Result<Self, AiError> {
        value.parse()
    }

    pub fn parse_all<I, S>(values: I) -> Result<Vec<Self>, AiError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        values
            .into_iter()
            .map(|value| value.as_ref().parse())
            .collect()
    }
}

impl FromStr for AiScope {
    type Err = AiError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim() {
            "catalog:read" => Ok(Self::CatalogRead),
            "cart:read" => Ok(Self::CartRead),
            "cart:write" => Ok(Self::CartWrite),
            "checkout:preview" => Ok(Self::CheckoutPreview),
            "order:create" => Ok(Self::OrderCreate),
            "order:read" => Ok(Self::OrderRead),
            "order:auto_purchase" => Ok(Self::AutoPurchase),
            _ => Err(AiError::UnknownScope),
        }
    }
}

impl fmt::Display for AiScope {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_api_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AiClientId(String);

impl AiClientId {
    pub fn parse(value: impl AsRef<str>) -> Result<Self, AiError> {
        let value = value.as_ref().trim();
        if value.is_empty() {
            return Err(AiError::BlankClientId);
        }

        Ok(Self(value.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for AiClientId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Authorization {
    id: AiAuthorizationId,
    subject_user_id: UserId,
    client_id: AiClientId,
    scopes: BTreeSet<AiScope>,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl Authorization {
    pub fn new(
        subject_user_id: UserId,
        client_id: AiClientId,
        scopes: impl IntoIterator<Item = AiScope>,
        expires_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: AiAuthorizationId::new(),
            subject_user_id,
            client_id,
            scopes: scopes.into_iter().collect(),
            expires_at,
            revoked_at: None,
        }
    }

    pub const fn id(&self) -> AiAuthorizationId {
        self.id
    }

    pub const fn subject_user_id(&self) -> UserId {
        self.subject_user_id
    }

    pub const fn client_id(&self) -> &AiClientId {
        &self.client_id
    }

    pub fn scopes(&self) -> impl Iterator<Item = AiScope> + '_ {
        self.scopes.iter().copied()
    }

    pub const fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    pub const fn revoked_at(&self) -> Option<DateTime<Utc>> {
        self.revoked_at
    }

    pub fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    pub const fn is_revoked(&self) -> bool {
        self.revoked_at.is_some()
    }

    /// Safety: `OrderCreate` never grants `AutoPurchase`. Each scope is checked exactly.
    pub fn has_scope(&self, scope: AiScope) -> bool {
        self.scopes.contains(&scope)
    }

    pub fn require_scope(&self, scope: AiScope, now: DateTime<Utc>) -> Result<(), AiError> {
        if self.is_revoked() {
            return Err(AiError::AuthorizationRevoked);
        }
        if self.is_expired(now) {
            return Err(AiError::AuthorizationExpired);
        }
        if !self.has_scope(scope) {
            return Err(AiError::MissingScope { required: scope });
        }

        Ok(())
    }

    pub fn revoke(&mut self, revoked_at: DateTime<Utc>) {
        if self.revoked_at.is_none() {
            self.revoked_at = Some(revoked_at);
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use market_bot_shared::UserId;

    use super::{AiClientId, AiScope, Authorization};

    #[test]
    fn should_parse_api_scope_strings() {
        assert_eq!(
            AiScope::parse("cart:write").expect("scope should parse"),
            AiScope::CartWrite
        );
        assert_eq!(
            AiScope::parse("order:auto_purchase").expect("scope should parse"),
            AiScope::AutoPurchase
        );
        assert!(AiScope::parse("order:*").is_err());
    }

    #[test]
    fn should_not_let_order_create_inherit_auto_purchase() {
        let authorization = Authorization::new(
            UserId::new(),
            AiClientId::parse("client-1").expect("client should parse"),
            [AiScope::OrderCreate],
            Utc::now() + Duration::hours(1),
        );

        assert!(authorization.has_scope(AiScope::OrderCreate));
        assert!(!authorization.has_scope(AiScope::AutoPurchase));
        assert!(matches!(
            authorization.require_scope(AiScope::AutoPurchase, Utc::now()),
            Err(crate::AiError::MissingScope {
                required: AiScope::AutoPurchase
            })
        ));
    }

    #[test]
    fn should_reject_expired_and_revoked_authorizations() {
        let now = Utc::now();
        let expired = Authorization::new(
            UserId::new(),
            AiClientId::parse("client-1").expect("client should parse"),
            [AiScope::CartWrite],
            now - Duration::minutes(1),
        );
        assert!(matches!(
            expired.require_scope(AiScope::CartWrite, now),
            Err(crate::AiError::AuthorizationExpired)
        ));

        let mut revoked = Authorization::new(
            UserId::new(),
            AiClientId::parse("client-1").expect("client should parse"),
            [AiScope::CartWrite],
            now + Duration::hours(1),
        );
        revoked.revoke(now);
        assert!(matches!(
            revoked.require_scope(AiScope::CartWrite, now),
            Err(crate::AiError::AuthorizationRevoked)
        ));
    }
}
