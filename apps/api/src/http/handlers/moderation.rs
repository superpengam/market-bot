use axum::{
    Extension, Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{DateTime, Utc};
use market_bot_moderation::{
    CreateReportInput, ModerationCaseStatus, ModerationDecision, ModerationError, ModerationReason,
    ModerationSubjectType, ReportId,
};
use market_bot_shared::{ApiError, ErrorCode, ProductId, RequestContext, UserId};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::AppState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActorRole {
    Admin,
    User,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct AuthenticatedActor {
    user_id: UserId,
    role: ActorRole,
}

#[derive(Debug, Deserialize)]
pub struct ReviewProductRequest {
    pub decision: ModerationDecision,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct SuspendProductRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub subject_type: ModerationSubjectType,
    pub subject_id: Uuid,
    pub reason_code: String,
    pub details: String,
}

#[derive(Debug, Deserialize)]
pub struct ResolveReportRequest {
    pub decision: ModerationDecision,
    pub reason: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReviewProductResponse {
    pub product_id: ProductId,
    pub decision: ModerationDecision,
    pub reason: String,
    pub actor_id: UserId,
    pub acted_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateReportResponse {
    pub moderation_case_id: Uuid,
    pub report_id: ReportId,
    pub status: ModerationCaseStatus,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ResolveReportResponse {
    pub report_id: ReportId,
    pub decision: ModerationDecision,
    pub reason: String,
    pub actor_id: UserId,
    pub acted_at: DateTime<Utc>,
}

pub async fn review_product(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(product_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<ReviewProductRequest>,
) -> Result<Json<ReviewProductResponse>, (StatusCode, Json<ApiError>)> {
    let actor = require_admin(&headers, context.request_id())?;
    let product_id = ProductId::from_uuid(product_id);
    let scope = format!("admin.products.reviews:{product_id}");
    if let Some(response) = replay_if_present(
        &state,
        actor.user_id,
        &scope,
        context.idempotency_key(),
        context.request_id(),
    )
    .await?
    {
        return Ok(Json(response));
    }
    let reason = parse_reason(actor.user_id, body.reason, context.request_id())?;
    state
        .moderation
        .review_product(product_id, body.decision, reason.clone())
        .await
        .map_err(|error| map_moderation_error(error, context.request_id()))?;
    let Json(response) = review_response(
        &state,
        product_id,
        body.decision,
        reason,
        context.request_id(),
    )
    .await?;
    remember_idempotent(
        &state,
        actor.user_id,
        &scope,
        context.idempotency_key(),
        &response,
        context.request_id(),
    )
    .await?;
    Ok(Json(response))
}

pub async fn suspend_product(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(product_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<SuspendProductRequest>,
) -> Result<Json<ReviewProductResponse>, (StatusCode, Json<ApiError>)> {
    let actor = require_admin(&headers, context.request_id())?;
    let product_id = ProductId::from_uuid(product_id);
    let scope = format!("admin.products.suspend:{product_id}");
    if let Some(response) = replay_if_present(
        &state,
        actor.user_id,
        &scope,
        context.idempotency_key(),
        context.request_id(),
    )
    .await?
    {
        return Ok(Json(response));
    }
    let reason = parse_reason(actor.user_id, body.reason, context.request_id())?;
    state
        .moderation
        .review_product(product_id, ModerationDecision::Suspended, reason.clone())
        .await
        .map_err(|error| map_moderation_error(error, context.request_id()))?;
    let Json(response) = review_response(
        &state,
        product_id,
        ModerationDecision::Suspended,
        reason,
        context.request_id(),
    )
    .await?;
    remember_idempotent(
        &state,
        actor.user_id,
        &scope,
        context.idempotency_key(),
        &response,
        context.request_id(),
    )
    .await?;
    Ok(Json(response))
}

pub async fn create_report(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    headers: HeaderMap,
    Json(body): Json<CreateReportRequest>,
) -> Result<Json<CreateReportResponse>, (StatusCode, Json<ApiError>)> {
    let actor = require_actor(&headers, context.request_id())?;
    let scope = "reports.create";
    if let Some(response) = replay_if_present(
        &state,
        actor.user_id,
        scope,
        context.idempotency_key(),
        context.request_id(),
    )
    .await?
    {
        return Ok(Json(response));
    }
    let case = state
        .moderation
        .create_report(CreateReportInput {
            reporter_id: actor.user_id,
            subject_type: body.subject_type,
            subject_id: body.subject_id,
            reason_code: body.reason_code,
            details: body.details,
        })
        .await
        .map_err(|error| map_moderation_error(error, context.request_id()))?;
    let report_id = case
        .report_id()
        .ok_or_else(|| internal_error(context.request_id(), "report case is missing report_id"))?;
    let response = CreateReportResponse {
        moderation_case_id: case.id().as_uuid(),
        report_id,
        status: case.status(),
    };
    remember_idempotent(
        &state,
        actor.user_id,
        scope,
        context.idempotency_key(),
        &response,
        context.request_id(),
    )
    .await?;

    Ok(Json(response))
}

pub async fn resolve_report(
    State(state): State<AppState>,
    Extension(context): Extension<RequestContext>,
    Path(report_id): Path<Uuid>,
    headers: HeaderMap,
    Json(body): Json<ResolveReportRequest>,
) -> Result<Json<ResolveReportResponse>, (StatusCode, Json<ApiError>)> {
    let actor = require_admin(&headers, context.request_id())?;
    let report_id = ReportId::from_uuid(report_id);
    let scope = format!("admin.reports.resolve:{report_id}");
    if let Some(response) = replay_if_present(
        &state,
        actor.user_id,
        &scope,
        context.idempotency_key(),
        context.request_id(),
    )
    .await?
    {
        return Ok(Json(response));
    }
    let reason = parse_reason(actor.user_id, body.reason, context.request_id())?;
    let case = state
        .moderation
        .resolve_report(report_id, body.decision, reason)
        .await
        .map_err(|error| map_moderation_error(error, context.request_id()))?;
    let action = case.latest_action().ok_or_else(|| {
        internal_error(
            context.request_id(),
            "resolved report is missing an audit action",
        )
    })?;
    let response = ResolveReportResponse {
        report_id,
        decision: body.decision,
        reason: action.reason().to_owned(),
        actor_id: action.actor_id(),
        acted_at: action.acted_at(),
    };
    remember_idempotent(
        &state,
        actor.user_id,
        &scope,
        context.idempotency_key(),
        &response,
        context.request_id(),
    )
    .await?;

    Ok(Json(response))
}

async fn review_response(
    state: &AppState,
    product_id: ProductId,
    decision: ModerationDecision,
    reason: ModerationReason,
    request_id: Uuid,
) -> Result<Json<ReviewProductResponse>, (StatusCode, Json<ApiError>)> {
    let case = state
        .moderation
        .product_review_case(product_id)
        .await
        .map_err(|error| map_moderation_error(error, request_id))?
        .ok_or_else(|| internal_error(request_id, "review did not create a moderation case"))?;
    let action = case
        .latest_action()
        .ok_or_else(|| internal_error(request_id, "review is missing an audit action"))?;

    Ok(Json(ReviewProductResponse {
        product_id,
        decision,
        reason: reason.text().to_owned(),
        actor_id: action.actor_id(),
        acted_at: action.acted_at(),
    }))
}

async fn replay_if_present<T>(
    state: &AppState,
    actor_id: UserId,
    scope: &str,
    key: Option<&str>,
    request_id: Uuid,
) -> Result<Option<T>, (StatusCode, Json<ApiError>)>
where
    T: serde::de::DeserializeOwned,
{
    let Some(key) = key else {
        return Ok(None);
    };
    let stored = state
        .moderation
        .idempotent_result(actor_id, scope, key)
        .await
        .map_err(|error| map_moderation_error(error, request_id))?;
    stored
        .map(|value| {
            serde_json::from_value(value)
                .map_err(|_| internal_error(request_id, "stored idempotent result is invalid"))
        })
        .transpose()
}

async fn remember_idempotent<T>(
    state: &AppState,
    actor_id: UserId,
    scope: &str,
    key: Option<&str>,
    value: &T,
    request_id: Uuid,
) -> Result<(), (StatusCode, Json<ApiError>)>
where
    T: serde::Serialize,
{
    let Some(key) = key else {
        return Ok(());
    };
    let stored = serde_json::to_value(value)
        .map_err(|_| internal_error(request_id, "failed to store idempotent result"))?;
    state
        .moderation
        .store_idempotent_result(actor_id, scope, key, stored)
        .await
        .map_err(|error| map_moderation_error(error, request_id))
}

fn parse_reason(
    actor_id: UserId,
    reason: String,
    request_id: Uuid,
) -> Result<ModerationReason, (StatusCode, Json<ApiError>)> {
    ModerationReason::new(actor_id, reason).map_err(|error| map_moderation_error(error, request_id))
}

fn require_admin(
    headers: &HeaderMap,
    request_id: Uuid,
) -> Result<AuthenticatedActor, (StatusCode, Json<ApiError>)> {
    let actor = require_actor(headers, request_id)?;
    if actor.role != ActorRole::Admin {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            ErrorCode::Forbidden,
            "admin authorization is required",
            request_id,
        ));
    }

    Ok(actor)
}

fn require_actor(
    headers: &HeaderMap,
    request_id: Uuid,
) -> Result<AuthenticatedActor, (StatusCode, Json<ApiError>)> {
    let Some(header) = headers.get(axum::http::header::AUTHORIZATION) else {
        return Err(unauthorized(request_id));
    };
    let Ok(value) = header.to_str() else {
        return Err(unauthorized(request_id));
    };
    let Some(token) = value.strip_prefix("Bearer ") else {
        return Err(unauthorized(request_id));
    };
    let Some((role, actor_id)) = token.split_once('/') else {
        return Err(unauthorized(request_id));
    };
    let Ok(actor_id) = Uuid::parse_str(actor_id) else {
        return Err(unauthorized(request_id));
    };
    let role = match role {
        "admin" => ActorRole::Admin,
        "user" => ActorRole::User,
        _ => return Err(unauthorized(request_id)),
    };

    Ok(AuthenticatedActor {
        user_id: UserId::from_uuid(actor_id),
        role,
    })
}

fn map_moderation_error(error: ModerationError, request_id: Uuid) -> (StatusCode, Json<ApiError>) {
    let (status, code) = match error {
        ModerationError::BlankReason
        | ModerationError::BlankReportDetails
        | ModerationError::BlankReasonCode
        | ModerationError::ProductNotReady
        | ModerationError::InvalidProductStatus(_) => {
            (StatusCode::BAD_REQUEST, ErrorCode::InvalidInput)
        }
        ModerationError::ProductNotFound
        | ModerationError::ReportNotFound
        | ModerationError::CaseNotFound => (StatusCode::NOT_FOUND, ErrorCode::NotFound),
        ModerationError::ProductNotPurchasable => (StatusCode::FORBIDDEN, ErrorCode::Forbidden),
        ModerationError::Catalog(_)
        | ModerationError::Search(_)
        | ModerationError::Outbox(_)
        | ModerationError::Scanner(_)
        | ModerationError::Repository(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, ErrorCode::InternalError)
        }
    };

    error_response(status, code, error.to_string(), request_id)
}

fn unauthorized(request_id: Uuid) -> (StatusCode, Json<ApiError>) {
    error_response(
        StatusCode::UNAUTHORIZED,
        ErrorCode::Unauthorized,
        "authentication is required",
        request_id,
    )
}

fn internal_error(request_id: Uuid, message: &str) -> (StatusCode, Json<ApiError>) {
    error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        ErrorCode::InternalError,
        message,
        request_id,
    )
}

fn error_response(
    status: StatusCode,
    code: ErrorCode,
    message: impl Into<String>,
    request_id: Uuid,
) -> (StatusCode, Json<ApiError>) {
    (status, Json(ApiError::new(code, message, request_id)))
}
