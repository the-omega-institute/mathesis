//! `/api/v1` resource routes.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{FromRequest, FromRequestParts, State},
    response::IntoResponse,
    routing::{get, put},
};
use mathesis_core::{
    app::App,
    ids::PersonId,
    model::{Role, Stage},
    policy::Policy,
};
use serde::{Deserialize, Serialize};

use crate::{auth::AuthenticatedCaller, problems::Problem};

mod bindings;
mod cma;
mod deposits;
mod feedback;
mod network;
mod papers;
mod research;
mod review;
mod solving;

pub(crate) type AppState = State<Arc<App>>;
pub(crate) type ApiResult<T> = Result<T, Problem>;

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(Problem))]
pub(crate) struct Body<T>(pub(crate) T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(Problem))]
pub(crate) struct Path<T>(pub(crate) T);

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Query), rejection(Problem))]
pub(crate) struct Query<T>(pub(crate) T);

pub(crate) fn routes() -> Router<Arc<App>> {
    Router::new()
        .merge(solving::routes())
        .merge(papers::routes())
        .merge(review::routes())
        .merge(research::routes())
        .merge(bindings::routes())
        .merge(feedback::routes())
        .merge(network::routes())
        .merge(cma::routes())
        .merge(deposits::routes())
        .route(
            "/openapi.json",
            get(|| async {
                axum::response::Response::builder()
                    .header("content-type", "application/json")
                    .body(axum::body::Body::from(include_str!("openapi.json")))
                    .unwrap()
            }),
        )
        .route("/research-graph", get(research_graph))
        .route("/knowledge/capabilities", get(knowledge_capabilities))
        .route("/policy", get(policy))
        .route("/me", get(me))
        .route("/people", get(list_people))
        .route("/people/{id}/roles", put(set_roles))
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PageQuery {
    pub(crate) limit: Option<u32>,
    pub(crate) before: Option<String>,
}

#[derive(Serialize)]
struct StageDescription {
    stage: Stage,
    code: &'static str,
    title: &'static str,
    description: &'static str,
}

#[derive(Serialize)]
struct PolicyDocument<'a> {
    policy: &'a Policy,
    stages: Vec<StageDescription>,
}

const STAGES: [(Stage, &str, &str); 4] = [
    (
        Stage::Hygiene,
        "Source",
        "The LaTeX source is read, the PDF compiles, and AI use is disclosed.",
    ),
    (
        Stage::Claims,
        "Statements",
        "The author confirms the theorems, lemmas and conjectures read from the source and marks the main results.",
    ),
    (
        Stage::Literature,
        "Literature",
        "No main result is already stated in, or directly implied by, prior work.",
    ),
    (
        Stage::Escape,
        "Escape analysis",
        "Each proved statement is judged content, bind-only or undetermined against explicit prerequisites. Live intermediate or conclusion witnesses, literature evidence and mathematical utility are assessed separately.",
    ),
];

async fn knowledge_capabilities(State(app): AppState) -> impl IntoResponse {
    Json(app.knowledge_capabilities())
}

async fn policy(State(app): AppState) -> impl IntoResponse {
    let stages = STAGES
        .iter()
        .map(|(stage, title, description)| StageDescription {
            stage: *stage,
            code: stage.code(),
            title,
            description,
        })
        .collect();
    Json(
        serde_json::to_value(PolicyDocument {
            policy: app.policy(),
            stages,
        })
        .unwrap_or_default(),
    )
}

async fn me(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.me(&caller).await?))
}

async fn list_people(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Query(q): Query<PageQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.list_people(&caller, q.limit, q.before).await?))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RolesBody {
    roles: Vec<Role>,
}

async fn set_roles(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Body(body): Body<RolesBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.set_roles(&caller, &PersonId(id), &body.roles).await?,
    ))
}

#[cfg(test)]
mod tests;

async fn research_graph(
    State(app): AppState,
    Query(query): Query<mathesis_core::app::ResearchGraphQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok((
        [(axum::http::header::CACHE_CONTROL, "no-store")],
        Json(app.research_graph(query).await?),
    ))
}
