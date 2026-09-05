use chrono::{DateTime, Utc};
use market_bot_shared::{ProductId, UserId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ModerationError;

macro_rules! define_id {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize,
        )]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

define_id!(ModerationCaseId);
define_id!(ModerationActionId);
define_id!(ReportId);
define_id!(FileScanResultId);

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationDecision {
    Approved,
    Rejected,
    Suspended,
    NeedsReview,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationSubjectType {
    Product,
    Seller,
    User,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationCaseKind {
    ProductReview,
    UserReport,
    FileScan,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModerationCaseStatus {
    Open,
    InReview,
    Resolved,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    Open,
    InReview,
    Resolved,
    Rejected,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanVerdict {
    Passed,
    Failed,
    NeedsReview,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModerationReason {
    actor_id: UserId,
    text: String,
}

impl ModerationReason {
    pub fn new(actor_id: UserId, text: impl Into<String>) -> Result<Self, ModerationError> {
        let text = sanitize_audit_text(text.into().trim());
        if text.is_empty() {
            return Err(ModerationError::BlankReason);
        }

        Ok(Self { actor_id, text })
    }

    pub const fn actor_id(&self) -> UserId {
        self.actor_id
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModerationAction {
    id: ModerationActionId,
    actor_id: UserId,
    decision: ModerationDecision,
    reason: String,
    acted_at: DateTime<Utc>,
}

impl ModerationAction {
    pub fn new(
        actor_id: UserId,
        decision: ModerationDecision,
        reason: impl Into<String>,
        acted_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: ModerationActionId::new(),
            actor_id,
            decision,
            reason: sanitize_audit_text(&reason.into()),
            acted_at,
        }
    }

    pub const fn id(&self) -> ModerationActionId {
        self.id
    }

    pub const fn actor_id(&self) -> UserId {
        self.actor_id
    }

    pub const fn decision(&self) -> ModerationDecision {
        self.decision
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub const fn acted_at(&self) -> DateTime<Utc> {
        self.acted_at
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ModerationCase {
    id: ModerationCaseId,
    case_kind: ModerationCaseKind,
    subject_type: ModerationSubjectType,
    subject_id: Uuid,
    report_id: Option<ReportId>,
    status: ModerationCaseStatus,
    decision: Option<ModerationDecision>,
    reason: Option<String>,
    actor_id: Option<UserId>,
    created_at: DateTime<Utc>,
    resolved_at: Option<DateTime<Utc>>,
    actions: Vec<ModerationAction>,
}

impl ModerationCase {
    pub fn open_product_review(product_id: ProductId, now: DateTime<Utc>) -> Self {
        Self {
            id: ModerationCaseId::new(),
            case_kind: ModerationCaseKind::ProductReview,
            subject_type: ModerationSubjectType::Product,
            subject_id: product_id.as_uuid(),
            report_id: None,
            status: ModerationCaseStatus::Open,
            decision: None,
            reason: None,
            actor_id: None,
            created_at: now,
            resolved_at: None,
            actions: Vec::new(),
        }
    }

    pub fn open_report(
        report_id: ReportId,
        subject_type: ModerationSubjectType,
        subject_id: Uuid,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id: ModerationCaseId::new(),
            case_kind: ModerationCaseKind::UserReport,
            subject_type,
            subject_id,
            report_id: Some(report_id),
            status: ModerationCaseStatus::Open,
            decision: None,
            reason: None,
            actor_id: None,
            created_at: now,
            resolved_at: None,
            actions: Vec::new(),
        }
    }

    pub const fn id(&self) -> ModerationCaseId {
        self.id
    }

    pub const fn case_kind(&self) -> ModerationCaseKind {
        self.case_kind
    }

    pub const fn subject_type(&self) -> ModerationSubjectType {
        self.subject_type
    }

    pub const fn subject_id(&self) -> Uuid {
        self.subject_id
    }

    pub const fn report_id(&self) -> Option<ReportId> {
        self.report_id
    }

    pub const fn status(&self) -> ModerationCaseStatus {
        self.status
    }

    pub const fn decision(&self) -> Option<ModerationDecision> {
        self.decision
    }

    pub fn reason(&self) -> Option<&str> {
        self.reason.as_deref()
    }

    pub const fn actor_id(&self) -> Option<UserId> {
        self.actor_id
    }

    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub const fn resolved_at(&self) -> Option<DateTime<Utc>> {
        self.resolved_at
    }

    pub fn actions(&self) -> &[ModerationAction] {
        &self.actions
    }

    pub fn latest_action(&self) -> Option<&ModerationAction> {
        self.actions.last()
    }

    pub fn audit_text(&self) -> String {
        let mut parts = Vec::new();
        if let Some(reason) = &self.reason {
            parts.push(reason.clone());
        }
        for action in &self.actions {
            parts.push(action.reason.clone());
        }
        parts.join(" ")
    }

    pub fn record_decision(&mut self, reason: &ModerationReason, decision: ModerationDecision) {
        let acted_at = Utc::now();
        let action = ModerationAction::new(reason.actor_id(), decision, reason.text(), acted_at);
        self.decision = Some(decision);
        self.reason = Some(action.reason().to_owned());
        self.actor_id = Some(reason.actor_id());
        self.status = match decision {
            ModerationDecision::NeedsReview => ModerationCaseStatus::InReview,
            ModerationDecision::Approved
            | ModerationDecision::Rejected
            | ModerationDecision::Suspended => ModerationCaseStatus::Resolved,
        };
        self.resolved_at = Some(acted_at);
        self.actions.push(action);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanAsset {
    pub asset_id: Uuid,
    pub product_id: ProductId,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ScanResult {
    id: FileScanResultId,
    asset_id: Uuid,
    product_id: ProductId,
    filename: String,
    content_type: String,
    verdict: ScanVerdict,
    reason_code: String,
    scanned_at: DateTime<Utc>,
}

impl ScanResult {
    pub fn new(
        asset: &ScanAsset,
        verdict: ScanVerdict,
        reason_code: impl Into<String>,
        scanned_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id: FileScanResultId::new(),
            asset_id: asset.asset_id,
            product_id: asset.product_id,
            filename: asset.filename.clone(),
            content_type: asset.content_type.clone(),
            verdict,
            reason_code: reason_code.into(),
            scanned_at,
        }
    }

    pub const fn id(&self) -> FileScanResultId {
        self.id
    }

    pub const fn asset_id(&self) -> Uuid {
        self.asset_id
    }

    pub const fn product_id(&self) -> ProductId {
        self.product_id
    }

    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    pub const fn verdict(&self) -> ScanVerdict {
        self.verdict
    }

    pub fn reason_code(&self) -> &str {
        &self.reason_code
    }

    pub const fn scanned_at(&self) -> DateTime<Utc> {
        self.scanned_at
    }
}

/// Safety: audit, report, and review records must never persist raw card
/// secrets, complete payment tokens, or unnecessary personal addresses.
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
            if lowered.contains("address:") {
                let prefix_end = lowered.find("address:").expect("address marker exists");
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
    use market_bot_shared::UserId;

    use super::{ModerationReason, sanitize_audit_text};

    #[test]
    fn should_redact_card_numbers_tokens_and_addresses() {
        let sanitized = sanitize_audit_text(
            "card 4111111111111111 token tok_live_secret address: 99 Hidden Road",
        );

        assert!(!sanitized.contains("4111111111111111"));
        assert!(!sanitized.contains("tok_live_secret"));
        assert!(!sanitized.contains("99 Hidden Road"));
    }

    #[test]
    fn should_sanitize_reason_text_on_construction() {
        let reason = ModerationReason::new(
            UserId::new(),
            "card 4111111111111111 token tok_live_secret address: 99 Hidden Road",
        )
        .expect("reason should be accepted after sanitizing");

        assert!(!reason.text().contains("4111111111111111"));
        assert!(!reason.text().contains("tok_live_secret"));
        assert!(!reason.text().contains("99 Hidden Road"));
        assert!(reason.text().contains("[REDACTED_CARD]"));
        assert!(reason.text().contains("[REDACTED_TOKEN]"));
        assert!(reason.text().contains("[REDACTED_ADDRESS]"));
    }
}
