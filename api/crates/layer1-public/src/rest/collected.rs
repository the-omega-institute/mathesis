//! Projection of the trusted collection service. No review or acceptance shortcuts.
use super::{ApiResult, AppState, Path, Query};
use crate::{auth::AuthenticatedCaller, problems::Problem};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Multipart, State},
    response::IntoResponse,
    routing::{get, post},
};
use mathesis_core::{
    CoreError,
    app::{App, MAX_UPLOAD_BYTES, Upload},
    model::{CollectedArchiveInput, PaperSearch, PreReviewedDelivery, SubmissionKind},
};
use serde::Deserialize;
use std::sync::Arc;
pub(super) fn routes() -> Router<Arc<App>> {
    Router::new()
        .route(
            "/pre-reviewed-deliveries",
            post(deliver).layer(DefaultBodyLimit::max(2 * MAX_UPLOAD_BYTES + 256 * 1024)),
        )
        .route("/collected-papers", get(list))
        .route("/collected-papers/{id}", get(paper))
        .route("/collected-papers/{id}/archive", post(attach_archive))
        .route("/collected-papers/{id}/index", post(index))
        .route("/collected-papers/{id}/knowledge", get(knowledge))
        .route("/collected-papers/{id}/proofs", get(proofs))
        .route(
            "/collected-papers/{id}/results/{claim}/formalization",
            post(request_proof),
        )
        .route(
            "/collected-papers/{id}/plain-language/request",
            post(request_explanation),
        )
        .route("/collected-papers/{id}/files/{kind}", get(file))
}
async fn attach_archive(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Query(q): Query<Version>,
    Json(input): Json<CollectedArchiveInput>,
) -> ApiResult<impl IntoResponse> {
    let version = q
        .version
        .ok_or_else(|| Problem(CoreError::invalid("name the exact publication version")))?;
    Ok(Json(
        app.attach_collected_archive(&caller, &id, version, input)
            .await?,
    ))
}
async fn knowledge(
    State(app): AppState,
    Path(id): Path<String>,
    Query(q): Query<Version>,
) -> ApiResult<impl IntoResponse> {
    let version = q
        .version
        .ok_or_else(|| Problem(CoreError::invalid("name the exact publication version")))?;
    Ok(Json(app.collected_knowledge(&id, version).await?))
}
async fn index(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Query(q): Query<Version>,
) -> ApiResult<impl IntoResponse> {
    let version = q
        .version
        .ok_or_else(|| Problem(CoreError::invalid("name the exact publication version")))?;
    app.index_collected_paper(&caller, &id, version).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
async fn request_explanation(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Query(q): Query<Version>,
) -> ApiResult<impl IntoResponse> {
    let version = q
        .version
        .ok_or_else(|| Problem(CoreError::invalid("name the exact publication version")))?;
    Ok(Json(
        app.request_collected_explanation(&caller, &id, version)
            .await?,
    ))
}
async fn deliver(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    mut form: Multipart,
) -> ApiResult<impl IntoResponse> {
    let mut metadata = None;
    let mut pdf = None;
    let mut source = None;
    while let Some(field) = form
        .next_field()
        .await
        .map_err(|e| Problem(CoreError::invalid(e.to_string())))?
    {
        let name = field.name().unwrap_or_default().to_owned();
        if name == "metadata" && metadata.is_none() {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| Problem(CoreError::invalid(e.to_string())))?;
            if bytes.len() > 128 * 1024 {
                return Err(Problem(CoreError::invalid("metadata exceeds 128 KiB")));
            }
            metadata = Some(
                serde_json::from_slice::<PreReviewedDelivery>(&bytes)
                    .map_err(|e| Problem(CoreError::invalid(format!("metadata: {e}"))))?,
            );
        } else if (name == "pdf" && pdf.is_none()) || (name == "source" && source.is_none()) {
            let filename = field.file_name().unwrap_or_default().to_owned();
            let bytes = field
                .bytes()
                .await
                .map_err(|e| Problem(CoreError::invalid(e.to_string())))?
                .to_vec();
            let upload = Upload { filename, bytes };
            if name == "pdf" {
                pdf = Some(upload);
            } else {
                source = Some(upload);
            }
        } else {
            return Err(Problem(CoreError::invalid(
                "unexpected or duplicate collection field",
            )));
        }
    }
    let missing = || Problem(CoreError::invalid("attach metadata, pdf and source"));
    Ok(Json(
        app.collect_pre_reviewed(
            &caller,
            metadata.ok_or_else(missing)?,
            pdf.ok_or_else(missing)?,
            source.ok_or_else(missing)?,
        )
        .await?,
    ))
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Version {
    version: Option<u32>,
}
async fn paper(
    State(app): AppState,
    Path(id): Path<String>,
    Query(q): Query<Version>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.collected_paper(&id, q.version).await?))
}
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    q: Option<String>,
    author: Option<String>,
    msc: Option<String>,
    kind: Option<SubmissionKind>,
    limit: Option<u32>,
    before: Option<String>,
}
async fn list(State(app): AppState, Query(q): Query<Search>) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.collected_papers(
            PaperSearch {
                q: q.q,
                author: q.author,
                msc: q.msc,
                kind: q.kind,
            },
            q.limit,
            q.before,
        )
        .await?,
    ))
}
async fn file(
    State(app): AppState,
    Path((id, kind)): Path<(String, String)>,
    Query(q): Query<Version>,
) -> ApiResult<impl IntoResponse> {
    let version = q
        .version
        .ok_or_else(|| Problem(CoreError::invalid("file requests require an exact version")))?;
    let file = app.collected_file(&id, version, &kind).await?;
    Ok(super::papers::file_response(file, kind == "pdf"))
}

async fn proofs(
    State(app): AppState,
    Path(id): Path<String>,
    Query(q): Query<Version>,
) -> ApiResult<impl IntoResponse> {
    let version = q
        .version
        .ok_or_else(|| Problem(CoreError::invalid("name the exact publication version")))?;
    Ok(Json(app.collected_proofs(&id, version).await?))
}
async fn request_proof(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path((id, claim)): Path<(String, String)>,
    Query(q): Query<Version>,
) -> ApiResult<impl IntoResponse> {
    let version = q
        .version
        .ok_or_else(|| Problem(CoreError::invalid("name the exact publication version")))?;
    app.request_collected_proof(&caller, &id, version, &claim.into())
        .await?;
    Ok(axum::http::StatusCode::ACCEPTED)
}
