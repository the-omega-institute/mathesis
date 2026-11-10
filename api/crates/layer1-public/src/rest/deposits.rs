use super::{ApiResult, AppState, Body, PageQuery, Path, Query};
use crate::auth::AuthenticatedCaller;
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::{StatusCode, header},
    response::IntoResponse,
    routing::{get, post},
};
use mathesis_core::{
    app::App,
    ids::{ClaimId, RecordId},
    model::NewLeanDeposit,
};
use serde::Deserialize;
use std::sync::Arc;
pub(super) fn routes() -> Router<Arc<App>> {
    Router::new()
        .route(
            "/formalizations",
            get(list)
                .post(submit)
                .layer(DefaultBodyLimit::max(64 * 1024 * 1024)),
        )
        .route("/formalizations/opportunities", get(opportunities))
        .route(
            "/formalizations/preflight",
            post(preflight).layer(DefaultBodyLimit::max(64 * 1024 * 1024)),
        )
        .route("/formalizations/{id}", get(detail))
        .route("/formalizations/{id}/project", get(project))
        .route("/formalizations/{id}/source", get(source))
        .route(
            "/formalizations/{id}/attempts",
            post(verify).layer(DefaultBodyLimit::max(8 * 1024 * 1024)),
        )
}
async fn list(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Query(q): Query<PageQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.lean_deposits(&c, q.limit, q.before).await?))
}
async fn submit(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Body(body): Body<NewLeanDeposit>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        StatusCode::CREATED,
        Json(app.submit_lean_deposit(&c, body).await?),
    ))
}
async fn detail(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.lean_deposit(&c, &id).await?))
}
async fn source(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        [
            (header::CONTENT_TYPE, "text/plain; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=Solution.lean",
            ),
            (header::CACHE_CONTROL, "private, no-store"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        app.lean_deposit_source(&c, &id).await?,
    ))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Target {
    record: RecordId,
    claim: ClaimId,
    expected_target: Option<String>,
    solution: Option<String>,
}
async fn verify(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Path(id): Path<String>,
    Body(target): Body<Target>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        StatusCode::ACCEPTED,
        Json(
            app.verify_lean_project(
                &c,
                &id,
                &target.record,
                &target.claim,
                target.expected_target.as_deref(),
                target.solution,
            )
            .await?,
        ),
    ))
}

async fn opportunities(
    State(app): AppState,
    Query(query): Query<mathesis_core::app::FormalizationQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.formalization_opportunities(query).await?))
}
async fn project(
    State(app): AppState,
    AuthenticatedCaller(c): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let deposit = app.lean_deposit(&c, &id).await?;
    let solution = app.lean_deposit_source(&c, &id).await?;
    let modules = app.lean_deposit_modules(&c, &id).await?;
    Ok((
        [
            (header::CACHE_CONTROL, "private, no-store"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=mathesis-project.json",
            ),
        ],
        Json(serde_json::json!({
            "title": deposit.title, "authors": deposit.authors, "formalizers": deposit.formalizers,
            "statement": deposit.statement, "source_reference": deposit.source_reference,
            "source_revision": deposit.source_revision, "toolchain": deposit.toolchain, "license": deposit.license,
            "solution": solution, "modules": modules, "project_files": deposit.project_files
        })),
    ))
}

async fn preflight(
    State(app): State<Arc<App>>,
    AuthenticatedCaller(_): AuthenticatedCaller,
    Body(body): Body<NewLeanDeposit>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.lean_preflight(&body).await?))
}
