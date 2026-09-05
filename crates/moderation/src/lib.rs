//! Content moderation, product review, and report handling.

mod adapters;
mod application;
mod domain;
mod errors;
mod ports;

pub use adapters::{InMemoryModerationRepository, SandboxContentScanner};
pub use application::{
    CreateReportInput, ModerationService, PublishCheckFailure, PublishReadiness,
};
pub use domain::{
    FileScanResultId, ModerationAction, ModerationActionId, ModerationCase, ModerationCaseId,
    ModerationCaseKind, ModerationCaseStatus, ModerationDecision, ModerationReason,
    ModerationSubjectType, ReportId, ReportStatus, ScanAsset, ScanResult, ScanVerdict,
    sanitize_audit_text,
};
pub use errors::ModerationError;
pub use ports::ListingFacts;
pub use ports::{ContentScanner, ModerationRepository, ModerationRepositoryError, ScannerError};
