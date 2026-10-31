use super::{ApiResult, AppState, Body, Path, Query};
use crate::auth::AuthenticatedCaller;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, patch, post},
};
use mathesis_core::{
    app::App,
    ids::{ClaimId, RecordId, SubmissionId},
    model::Caller,
};
use serde::Deserialize;
use std::sync::Arc;
pub(super) fn routes() -> Router<Arc<App>> {
    Router::new()
        .route("/papers/{record}/statements/{claim}", get(statement))
        .route("/papers/{record}/statements/{claim}/target", get(target))
        .route(
            "/papers/{record}/statements/{claim}/work",
            get(work).put(control_work),
        )
        .route(
            "/papers/{record}/statements/{claim}/contributions",
            get(contributions)
                .post(contribute)
                .layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
        )
        .route(
            "/papers/{record}/statements/{claim}/reuse",
            post(reuse).layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
        )
        .route("/papers/{record}/statements/{claim}/uses", get(uses))
        .route("/contributions/{attempt}/bundle", get(contribution_bundle))
        .route(
            "/papers/{record}/statements/{claim}/attempts",
            get(verified)
                .post(attempt)
                .layer(DefaultBodyLimit::max(64 * 1024 * 1024)),
        )
        .route(
            "/papers/{record}/statements/{claim}/attempts/mine",
            get(mine),
        )
        .route(
            "/papers/{record}/statements/{claim}/attempts/{attempt}/bundle",
            get(bundle),
        )
        .route("/papers/{record}/errors", get(reports).post(report))
        .route("/errors/{id}", get(error_report))
        .route("/me/error-notifications", get(error_notifications))
        .route("/supporters", get(supporters))
        .route("/me/agents", get(agents).post(create_agent))
        .route("/me/agents/{id}", patch(update_agent).delete(retire_agent))
        .route("/me/notifications", get(notifications))
        .route("/conjectures/{record}/{claim}", get(conjecture))
        .route("/conjectures/{record}/{claim}/target", get(target))
        .route(
            "/conjectures/{record}/{claim}/attempts",
            get(verified)
                .post(attempt)
                .layer(DefaultBodyLimit::max(64 * 1024 * 1024)),
        )
        .route("/conjectures/{record}/{claim}/attempts/mine", get(mine))
        .route("/attempts/{id}", get(status))
        .route("/leaderboard", get(board))
        .route("/entrants/{id}", get(profile))
        .route(
            "/submissions/{id}/open-problems/check",
            post(check_problems),
        )
}
async fn agents(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.agents(&c).await?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentBody {
    name: String,
}
async fn create_agent(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Body(b): Body<AgentBody>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        StatusCode::CREATED,
        Json(app.create_agent(&c, b.name).await?),
    ))
}
async fn update_agent(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Path(id): Path<String>,
    Body(b): Body<AgentBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.update_agent(&c, &id, Some(b.name), false).await?))
}
async fn retire_agent(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.update_agent(&c, &id, None, true).await?))
}
async fn notifications(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.solve_notifications(&c).await?))
}
async fn check_problems(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    app.queue_open_problems(&c, &SubmissionId(id)).await?;
    Ok(StatusCode::ACCEPTED)
}
async fn target(
    State(app): AppState,
    Path((r, c)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    let target = app.target(&RecordId(r), &ClaimId(c)).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"Target.lean\"",
            ),
        ],
        target.lean,
    ))
}
async fn conjecture(
    State(app): AppState,
    Path((r, c)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.conjecture(&RecordId(r), &ClaimId(c)).await?))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AttemptBody {
    solution: String,
    #[serde(default)]
    as_agent: Option<String>,
    #[serde(default)]
    note: String,
}
async fn attempt(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path((r, c)): Path<(String, String)>,
    Body(b): Body<AttemptBody>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        StatusCode::ACCEPTED,
        Json(
            app.submit_attempt(
                &caller,
                &RecordId(r),
                &ClaimId(c),
                b.solution,
                b.as_agent,
                b.note,
            )
            .await?,
        ),
    ))
}
async fn mine(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path((r, c)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.my_attempts(&caller, &RecordId(r), &ClaimId(c)).await?,
    ))
}
fn caller(c: &Option<AuthenticatedCaller>) -> Option<&Caller> {
    c.as_ref().map(|c| &c.0)
}
async fn status(
    State(app): AppState,
    c: Option<AuthenticatedCaller>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.attempt(caller(&c), &id).await?))
}
async fn verified(
    State(app): AppState,
    Path((r, c)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.public_attempts(&RecordId(r), &ClaimId(c)).await?))
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoardQuery {
    period: Option<String>,
    entrants: Option<String>,
}
async fn board(State(app): AppState, Query(q): Query<BoardQuery>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.leaderboard(
            q.period.as_deref().unwrap_or("all"),
            q.entrants.as_deref().unwrap_or("all"),
        )
        .await?,
    ))
}
async fn profile(State(app): AppState, Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.entrant_profile(&id).await?))
}

async fn statement(
    State(app): AppState,
    Path((r, c)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.public_statement(&RecordId(r), &ClaimId(c)).await?))
}
async fn report(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Path(r): Path<String>,
    Body(b): Body<mathesis_core::model::NewErrorReport>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        StatusCode::ACCEPTED,
        Json(app.file_error_report(&c, &RecordId(r), b).await?),
    ))
}
async fn reports(
    State(app): AppState,
    c: Option<AuthenticatedCaller>,
    Path(r): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.error_reports(caller(&c), Some(&RecordId(r))).await?,
    ))
}
async fn error_report(
    State(app): AppState,
    c: Option<AuthenticatedCaller>,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.error_report(caller(&c), &id).await?))
}
async fn error_notifications(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.error_notifications(&c).await?))
}
async fn supporters(State(app): AppState) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.supporters().await?))
}

async fn bundle(
    State(app): AppState,
    Path((r, c, a)): Path<(String, String, String)>,
) -> ApiResult<impl IntoResponse> {
    let bundle = app
        .verification_bundle(&RecordId(r), &ClaimId(c), &a)
        .await?;
    Ok((
        [
            (header::CACHE_CONTROL, "no-store"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=mathesis-verification.json",
            ),
        ],
        Json(bundle),
    ))
}

async fn work(
    State(app): AppState,
    c: Option<AuthenticatedCaller>,
    Path((r, k)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(
            app.formalization_work(c.as_ref().map(|c| &c.0), &RecordId(r), &ClaimId(k))
                .await?,
        ),
    ))
}
async fn control_work(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path((r, c)): Path<(String, String)>,
    Body(b): Body<mathesis_core::model::FormalizationControl>,
) -> ApiResult<impl IntoResponse> {
    let record = RecordId(r);
    let claim = ClaimId(c);
    app.control_formalization(&caller, &record, &claim, b)
        .await?;
    Ok(Json(
        app.formalization_work(Some(&caller), &record, &claim)
            .await?,
    ))
}

async fn contributions(
    State(app): AppState,
    Path((r, c)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(app.checked_contributions(&RecordId(r), &ClaimId(c)).await?),
    ))
}
async fn contribute(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path((r, c)): Path<(String, String)>,
    Body(b): Body<mathesis_core::model::NewIntermediate>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        StatusCode::ACCEPTED,
        Json(
            app.submit_intermediate(&caller, &RecordId(r), &ClaimId(c), b)
                .await?,
        ),
    ))
}
async fn reuse(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path((r, c)): Path<(String, String)>,
    Body(b): Body<mathesis_core::model::ReusingAttempt>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        StatusCode::ACCEPTED,
        Json(
            app.submit_reusing_attempt(&caller, &RecordId(r), &ClaimId(c), b)
                .await?,
        ),
    ))
}
async fn uses(
    State(app): AppState,
    Path((r, c)): Path<(String, String)>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        [(header::CACHE_CONTROL, "no-store")],
        Json(app.result_uses(&RecordId(r), &ClaimId(c)).await?),
    ))
}
async fn contribution_bundle(
    State(app): AppState,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        [
            (header::CACHE_CONTROL, "no-store"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=mathesis-contribution.json",
            ),
        ],
        Json(app.checked_contribution(&id).await?),
    ))
}
