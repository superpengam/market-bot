use chrono::{DateTime, Utc};
use market_bot_shared::{AiActionId, AiAuthorizationId, OrderId, UserId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiActionType {
    Authorize,
    Revoke,
    SearchProducts,
    AddToCart,
    AutoPurchase,
}

impl AiActionType {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Authorize => "authorize",
            Self::Revoke => "revoke",
            Self::SearchProducts => "search_products",
            Self::AddToCart => "add_to_cart",
            Self::AutoPurchase => "auto_purchase",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AiActionResult {
    Succeeded,
    RequiresUserConfirmation,
    Blocked,
    Failed,
}

impl AiActionResult {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::RequiresUserConfirmation => "requires_user_confirmation",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
        }
    }

    pub const fn from_error(error: &crate::AiError) -> Self {
        match error {
            crate::AiError::RequiresUserConfirmation { .. } => Self::RequiresUserConfirmation,
            crate::AiError::PolicyBlocked { .. } => Self::Blocked,
            _ => Self::Failed,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AiAction {
    id: AiActionId,
    authorization_id: Option<AiAuthorizationId>,
    subject_user_id: Option<UserId>,
    action_type: AiActionType,
    input_summary: String,
    result: AiActionResult,
    request_id: Uuid,
    order_id: Option<OrderId>,
    error_code: Option<String>,
    created_at: DateTime<Utc>,
}

pub struct AiActionRecord {
    pub authorization_id: Option<AiAuthorizationId>,
    pub subject_user_id: Option<UserId>,
    pub action_type: AiActionType,
    pub input_summary: String,
    pub result: AiActionResult,
    pub request_id: Uuid,
    pub order_id: Option<OrderId>,
    pub error_code: Option<String>,
}

impl AiAction {
    pub fn new(record: AiActionRecord) -> Self {
        Self {
            id: AiActionId::new(),
            authorization_id: record.authorization_id,
            subject_user_id: record.subject_user_id,
            action_type: record.action_type,
            input_summary: sanitize_audit_text(&record.input_summary),
            result: record.result,
            request_id: record.request_id,
            order_id: record.order_id,
            error_code: record.error_code,
            created_at: Utc::now(),
        }
    }

    pub const fn id(&self) -> AiActionId {
        self.id
    }

    pub const fn authorization_id(&self) -> Option<AiAuthorizationId> {
        self.authorization_id
    }

    pub const fn subject_user_id(&self) -> Option<UserId> {
        self.subject_user_id
    }

    pub const fn action_type(&self) -> AiActionType {
        self.action_type
    }

    pub fn input_summary(&self) -> &str {
        &self.input_summary
    }

    pub const fn result(&self) -> AiActionResult {
        self.result
    }

    pub const fn request_id(&self) -> Uuid {
        self.request_id
    }

    pub const fn order_id(&self) -> Option<OrderId> {
        self.order_id
    }

    pub fn error_code(&self) -> Option<&str> {
        self.error_code.as_deref()
    }

    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}

/// Safety: audit records must never persist card numbers, payment tokens,
/// or unnecessary personal addresses.
pub fn sanitize_audit_text(value: &str) -> String {
    let without_cards = redact_card_numbers(value);
    let without_tokens = redact_payment_tokens(&without_cards);
    redact_personal_addresses(&without_tokens)
}

fn redact_card_numbers(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut digits = String::new();

    for character in value.chars() {
        if character.is_ascii_digit() {
            digits.push(character);
            continue;
        }

        flush_digits(&mut output, &digits);
        digits.clear();
        output.push(character);
    }

    flush_digits(&mut output, &digits);
    output
}

fn flush_digits(output: &mut String, digits: &str) {
    if (13..=19).contains(&digits.len()) {
        output.push_str("[REDACTED_CARD]");
    } else {
        output.push_str(digits);
    }
}

fn redact_payment_tokens(value: &str) -> String {
    value
        .split_whitespace()
        .map(|token| {
            let lowered = token.to_ascii_lowercase();
            if lowered.contains("tok_") || lowered.starts_with("sk_") || lowered.starts_with("pk_")
            {
                "[REDACTED_TOKEN]"
            } else {
                token
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn redact_personal_addresses(value: &str) -> String {
    value
        .split('\n')
        .map(|line| {
            let lowered = line.to_ascii_lowercase();
            if let Some(prefix_end) = lowered.find("address:") {
                let mut redacted = line[..prefix_end].to_owned();
                redacted.push_str("address: [REDACTED_ADDRESS]");
                redacted
            } else {
                line.to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::sanitize_audit_text;

    #[test]
    fn should_redact_card_numbers_tokens_and_addresses_from_audit_text() {
        let sanitized = sanitize_audit_text(
            "card 4111111111111111 token tok_live_secret address: 99 Hidden Road",
        );

        assert!(!sanitized.contains("4111111111111111"));
        assert!(!sanitized.contains("tok_live_secret"));
        assert!(!sanitized.contains("99 Hidden Road"));
    }
}
