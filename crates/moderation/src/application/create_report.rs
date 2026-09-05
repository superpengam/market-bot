use chrono::Utc;
use market_bot_catalog::CatalogRepository;
use market_bot_search::SearchRepository;
use market_bot_shared::{OutboxEvent, OutboxStore, ProductId, UserId};
use serde_json::json;
use uuid::Uuid;

use super::ModerationService;
use crate::{
    ModerationError,
    domain::{
        ModerationCase, ModerationDecision, ModerationReason, ModerationSubjectType, ReportId,
        ReportStatus, sanitize_audit_text,
    },
    ports::{ContentScanner, ModerationRepository, StoredReport},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreateReportInput {
    pub reporter_id: UserId,
    pub subject_type: ModerationSubjectType,
    pub subject_id: Uuid,
    pub reason_code: String,
    pub details: String,
}

impl<R, C, S, O, Sc> ModerationService<R, C, S, O, Sc>
where
    R: ModerationRepository,
    C: CatalogRepository,
    S: SearchRepository,
    O: OutboxStore,
    Sc: ContentScanner,
{
    pub async fn create_report(
        &self,
        input: CreateReportInput,
    ) -> Result<ModerationCase, ModerationError> {
        let reason_code = input.reason_code.trim().to_owned();
        if reason_code.is_empty() {
            return Err(ModerationError::BlankReasonCode);
        }

        let details = sanitize_audit_text(input.details.trim());
        if details.is_empty() {
            return Err(ModerationError::BlankReportDetails);
        }

        let report_id = ReportId::new();
        let mut case = ModerationCase::open_report(
            report_id,
            input.subject_type,
            input.subject_id,
            Utc::now(),
        );
        let report = StoredReport {
            report_id,
            reporter_id: input.reporter_id,
            subject_type: input.subject_type,
            subject_id: input.subject_id,
            reason_code: reason_code.clone(),
            details: details.clone(),
            status: ReportStatus::Open,
            case_id: case.id(),
        };
        case.record_decision(
            &ModerationReason::new(input.reporter_id, reason_code.clone())?,
            ModerationDecision::NeedsReview,
        );
        self.repository
            .save_case(case.clone())
            .await
            .map_err(ModerationError::Repository)?;
        self.repository
            .save_report(report)
            .await
            .map_err(ModerationError::Repository)?;
        self.outbox
            .append(OutboxEvent::new(
                "report.created",
                "report",
                report_id.as_uuid(),
                json!({
                    "report_id": report_id.as_uuid(),
                    "reporter_id": input.reporter_id,
                    "subject_type": input.subject_type,
                    "subject_id": input.subject_id,
                    "reason_code": reason_code,
                    "occurred_at": Utc::now(),
                }),
            ))
            .await
            .map_err(ModerationError::Outbox)?;
        Ok(case)
    }

    pub async fn resolve_report(
        &self,
        report_id: ReportId,
        decision: ModerationDecision,
        reason: ModerationReason,
    ) -> Result<ModerationCase, ModerationError> {
        let mut report = self
            .repository
            .find_report(report_id)
            .await
            .map_err(ModerationError::Repository)?
            .ok_or(ModerationError::ReportNotFound)?;
        let mut case = self
            .repository
            .find_report_case(report_id)
            .await
            .map_err(ModerationError::Repository)?
            .ok_or(ModerationError::CaseNotFound)?;

        case.record_decision(&reason, decision);
        report.status = match decision {
            ModerationDecision::Rejected => ReportStatus::Rejected,
            ModerationDecision::NeedsReview => ReportStatus::InReview,
            ModerationDecision::Approved | ModerationDecision::Suspended => ReportStatus::Resolved,
        };

        if decision == ModerationDecision::Suspended
            && report.subject_type == ModerationSubjectType::Product
        {
            self.review_product(
                ProductId::from_uuid(report.subject_id),
                decision,
                reason.clone(),
            )
            .await?;
        }

        self.repository
            .save_case(case.clone())
            .await
            .map_err(ModerationError::Repository)?;
        self.repository
            .save_report(report)
            .await
            .map_err(ModerationError::Repository)?;
        self.outbox
            .append(OutboxEvent::new(
                "report.resolved",
                "report",
                report_id.as_uuid(),
                json!({
                    "report_id": report_id.as_uuid(),
                    "decision": decision,
                    "reason": reason.text(),
                    "actor_id": reason.actor_id(),
                    "occurred_at": Utc::now(),
                }),
            ))
            .await
            .map_err(ModerationError::Outbox)?;
        Ok(case)
    }

    pub async fn report_details(&self, report_id: ReportId) -> Result<String, ModerationError> {
        let report = self
            .repository
            .find_report(report_id)
            .await
            .map_err(ModerationError::Repository)?
            .ok_or(ModerationError::ReportNotFound)?;
        Ok(report.details)
    }
}
