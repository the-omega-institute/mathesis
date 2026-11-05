//! The author's routes (upload, confirm, revise, visibility), the files of
//! a paper, and the public pages of accepted papers.

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, FromRequest, Multipart, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use mathesis_core::{
    CoreError,
    app::{App, MAX_UPLOAD_BYTES, PaperFile, SubmissionScope, Upload},
    ids::{RecordId, SubmissionId},
    model::{
        ClaimConfirmation, ConfirmManuscriptDelivery, NewManuscriptDelivery, NewPaper,
        PrepareSubmission, SubmissionPurpose, SubmitPrepared, Visibility,
    },
};
use serde::Deserialize;

use super::{ApiResult, AppState, Body, Path, Query};
use crate::{auth::AuthenticatedCaller, problems::Problem};

pub(super) fn routes() -> Router<Arc<App>> {
    let upload_limit = DefaultBodyLimit::max(MAX_UPLOAD_BYTES + 1024 * 1024);
    Router::new()
        .route(
            "/submissions",
            get(list_submissions).post(submit).layer(upload_limit),
        )
        .route(
            "/submissions/prepare",
            post(prepare_submission).layer(DefaultBodyLimit::max(
                MAX_UPLOAD_BYTES * 4 / 3 + 1024 * 1024,
            )),
        )
        .route("/submissions/{id}/submit", post(submit_prepared))
        .route("/submissions/{id}", get(get_submission))
        .route("/submissions/{id}/plain-language", put(record_explanation))
        .route(
            "/submissions/{id}/plain-language/request",
            post(request_explanation),
        )
        .route(
            "/manuscript-deliveries",
            get(find_manuscript)
                .post(deliver_manuscript)
                .layer(DefaultBodyLimit::max(
                    MAX_UPLOAD_BYTES * 4 / 3 + 1024 * 1024,
                )),
        )
        .route("/submissions/{id}/delivery", get(manuscript_receipt))
        .route(
            "/submissions/{id}/delivery-confirmation",
            post(confirm_manuscript),
        )
        .route("/submissions/{id}/review", get(review_report))
        .route("/submissions/{id}/review.md", get(review_markdown))
        .route("/submissions/{id}/claims", post(confirm_claims))
        .route(
            "/submissions/{id}/lean-statement/response",
            post(respond_lean_statement),
        )
        .route(
            "/submissions/{id}/versions",
            post(upload_version).layer(upload_limit),
        )
        .route("/submissions/{id}/withdraw", post(withdraw))
        .route("/submissions/{id}/visibility", put(set_visibility))
        .route("/submissions/{id}/contributors", put(set_contributors))
        .route("/submissions/{id}/analysis", get(analysis))
        .route("/submissions/{id}/files/{kind}", get(file))
        .route("/submissions/{id}/evidence", get(evidence_file))
        .route("/papers", get(list_papers))
        .route("/conjectures", get(list_conjectures))
        .route("/research-problems", get(list_research_problems))
        .route("/research-problems/{id}", get(research_problem))
        .route("/sitemap.xml", get(sitemap))
        .route("/papers/{id}", get(paper))
        .route("/papers/{id}/landing.html", get(paper_landing))
        .route("/papers/{id}/pdf", get(paper_pdf))
}

/// The parts of an upload form: `metadata` (JSON), `note` (text) and
/// `source` (the `.tex`, `.zip` or `.tar.gz`).
struct UploadForm {
    processing_consent: Option<String>,
    metadata: Option<String>,
    note: String,
    source: Option<Upload>,
}

async fn read_form(mut multipart: Multipart) -> Result<UploadForm, Problem> {
    let bad =
        |e: axum::extract::multipart::MultipartError| Problem(CoreError::Invalid(e.body_text()));
    let mut form = UploadForm {
        processing_consent: None,
        metadata: None,
        note: String::new(),
        source: None,
    };
    while let Some(field) = multipart.next_field().await.map_err(bad)? {
        match field.name().unwrap_or_default() {
            "processing_consent" => {
                form.processing_consent = Some(field.text().await.map_err(bad)?)
            }
            "metadata" => form.metadata = Some(field.text().await.map_err(bad)?),
            "note" => form.note = field.text().await.map_err(bad)?,
            "source" => {
                let filename = field.file_name().unwrap_or("source.tex").to_owned();
                let bytes = field.bytes().await.map_err(bad)?.to_vec();
                form.source = Some(Upload { filename, bytes });
            }
            other => {
                return Err(Problem(CoreError::Invalid(format!(
                    "unexpected form field {other:?}"
                ))));
            }
        }
    }
    Ok(form)
}

fn missing_source() -> Problem {
    Problem(CoreError::invalid("attach the LaTeX source as `source`"))
}

async fn deliver_manuscript(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    request: Request,
) -> ApiResult<impl IntoResponse> {
    if request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|v| v.trim() == "application/json")
        })
    {
        use base64::Engine;
        let Body(encoded) = Body::<EncodedManuscript>::from_request(request, &app).await?;
        if encoded.source_base64.len() > MAX_UPLOAD_BYTES * 4 / 3 {
            return Err(Problem(CoreError::invalid(
                "manuscript source exceeds 30 MiB",
            )));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&encoded.source_base64)
            .map_err(|_| Problem(CoreError::invalid("invalid source_base64")))?;
        return Ok(Json(
            app.deliver_manuscript(
                &caller,
                encoded.metadata,
                Upload {
                    filename: encoded.filename,
                    bytes,
                },
            )
            .await?,
        ));
    }
    let multipart = Multipart::from_request(request, &app)
        .await
        .map_err(|e| Problem(CoreError::invalid(e.to_string())))?;
    let form = read_form(multipart).await?;
    let metadata = form
        .metadata
        .ok_or_else(|| Problem(CoreError::invalid("attach manuscript delivery metadata")))?;
    let new: NewManuscriptDelivery = serde_json::from_str(&metadata)
        .map_err(|e| Problem(CoreError::invalid(format!("metadata: {e}"))))?;
    Ok(Json(
        app.deliver_manuscript(&caller, new, form.source.ok_or_else(missing_source)?)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EncodedManuscript {
    metadata: NewManuscriptDelivery,
    filename: String,
    source_base64: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FindManuscript {
    producer: String,
    work_id: String,
    purpose: SubmissionPurpose,
}

async fn find_manuscript(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Query(q): Query<FindManuscript>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.find_manuscript_delivery(&caller, &q.producer, &q.work_id, q.purpose)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeliveryVersion {
    version: Option<u32>,
}

async fn manuscript_receipt(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Query(q): Query<DeliveryVersion>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.manuscript_receipt(&caller, &SubmissionId(id), q.version)
            .await?,
    ))
}

async fn confirm_manuscript(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Body(body): Body<ConfirmManuscriptDelivery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.confirm_manuscript_delivery(&caller, &SubmissionId(id), body)
            .await?,
    ))
}

async fn submit(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    let form = read_form(multipart).await?;
    let metadata = form.metadata.ok_or_else(|| {
        Problem(CoreError::invalid(
            "attach the paper metadata as `metadata`",
        ))
    })?;
    let new: NewPaper = serde_json::from_str(&metadata)
        .map_err(|e| Problem(CoreError::Invalid(format!("metadata: {e}"))))?;
    let source = form.source.unwrap_or(Upload {
        filename: String::new(),
        bytes: vec![],
    });
    Ok((
        StatusCode::CREATED,
        Json(app.submit_paper(&caller, new, source).await?),
    ))
}

async fn upload_version(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    multipart: Multipart,
) -> ApiResult<impl IntoResponse> {
    let form = read_form(multipart).await?;
    let source = form.source.ok_or_else(missing_source)?;
    Ok(Json(
        app.upload_version(
            &caller,
            &SubmissionId(id),
            source,
            form.note,
            form.processing_consent,
        )
        .await?,
    ))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SubmissionQuery {
    scope: Option<SubmissionScope>,
    limit: Option<u32>,
    before: Option<String>,
}

async fn list_submissions(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Query(q): Query<SubmissionQuery>,
) -> ApiResult<impl IntoResponse> {
    let scope = q.scope.unwrap_or(SubmissionScope::Mine);
    Ok(Json(
        app.list_submissions(&caller, scope, q.limit, q.before)
            .await?,
    ))
}

async fn review_report(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let report = app.review_report(&caller, &SubmissionId(id)).await?;
    let mut response = Json(report).into_response();
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=mathesis-review.json"),
    );
    Ok(response)
}

async fn get_submission(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.submission(&caller, &SubmissionId(id)).await?))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfirmBody {
    claims: Vec<ClaimConfirmation>,
}

async fn confirm_claims(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Body(body): Body<ConfirmBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.confirm_claims(&caller, &SubmissionId(id), body.claims)
            .await?,
    ))
}

async fn withdraw(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.withdraw(&caller, &SubmissionId(id)).await?))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VisibilityBody {
    visibility: Visibility,
}

async fn set_visibility(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Body(body): Body<VisibilityBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.set_analysis_visibility(&caller, &SubmissionId(id), body.visibility)
            .await?,
    ))
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ContributorsBody {
    open: bool,
}

async fn set_contributors(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Body(body): Body<ContributorsBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.set_open_to_contributors(&caller, &SubmissionId(id), body.open)
            .await?,
    ))
}

async fn analysis(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.analysis(&caller, &SubmissionId(id)).await?))
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileQuery {
    version: Option<u32>,
}

/// `kind` is `pdf` (inline; public once accepted) or `source` (download;
/// authors and staff).
async fn file(
    State(app): AppState,
    caller: Option<AuthenticatedCaller>,
    Path((id, kind)): Path<(String, String)>,
    Query(q): Query<FileQuery>,
) -> ApiResult<Response> {
    let pdf = match kind.as_str() {
        "pdf" => true,
        "source" => false,
        _ => return Err(Problem(CoreError::not_found("file", kind))),
    };
    let file = app
        .paper_file(
            caller.as_ref().map(|c| &c.0),
            &SubmissionId(id),
            q.version,
            pdf,
        )
        .await?;
    Ok(file_response(file, pdf))
}

pub(super) fn file_response(file: PaperFile, inline: bool) -> Response {
    let filename: String = file
        .filename
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    let disposition = format!(
        "{}; filename=\"{}\"",
        if inline { "inline" } else { "attachment" },
        if filename.is_empty() {
            "paper"
        } else {
            &filename
        }
    );
    let mut response = file.bytes.into_response();
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(file.content_type),
    );
    if let Ok(value) = HeaderValue::from_str(&disposition) {
        headers.insert(header::CONTENT_DISPOSITION, value);
    }
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("sandbox"),
    );
    response
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct PaperQuery {
    limit: Option<u32>,
    before: Option<String>,
    q: Option<String>,
    author: Option<String>,
    msc: Option<String>,
    kind: Option<mathesis_core::model::SubmissionKind>,
}
async fn list_papers(
    State(app): AppState,
    Query(q): Query<PaperQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.search_papers(
            mathesis_core::model::PaperSearch {
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

async fn paper(State(app): AppState, Path(id): Path<String>) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.paper(&RecordId(id)).await?))
}

/// A stable record-based PDF URL for academic citations and crawlers.
async fn paper_pdf(State(app): AppState, Path(id): Path<String>) -> ApiResult<Response> {
    let paper = app.paper(&RecordId(id)).await?;
    if !paper.summary.public {
        return Err(Problem(CoreError::not_found(
            "paper",
            paper.summary.record.as_str(),
        )));
    }
    let file = app
        .paper_file(None, &paper.summary.submission, None, true)
        .await?;
    let mut response = file_response(file, true);
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    Ok(response)
}

/// A direct HTML response for Scholar and other academic crawlers. The
/// interactive React page remains the normal reader, while this response
/// contains all bibliographic metadata without requiring JavaScript.
async fn paper_landing(
    State(app): AppState,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    let paper = app.paper(&RecordId(id)).await?;
    if !paper.summary.public {
        return Err(Problem(CoreError::not_found(
            "paper",
            paper.summary.record.as_str(),
        )));
    }
    let origin = request_origin(&headers);
    let record = paper.summary.record.as_str();
    let landing_url = format!("{origin}/papers/{record}/landing.html");
    let pdf_url = format!("{origin}/papers/{record}/pdf");
    let has_pdf = paper.versions.iter().any(|version| version.has_pdf);
    let title = html_escape(&paper.summary.title);
    let abstract_text = html_escape(&paper.summary.abstract_text);
    let authors = paper
        .summary
        .authors
        .iter()
        .map(|author| {
            let name = html_escape(&author.name);
            format!("<meta name=\"citation_author\" content=\"{name}\">")
        })
        .collect::<Vec<_>>()
        .join("");
    let author_list = paper
        .summary
        .authors
        .iter()
        .map(|author| html_escape(&author.name))
        .collect::<Vec<_>>()
        .join(", ");
    let date = paper.summary.accepted_at.format("%Y-%m-%d");
    let kind = match paper.summary.kind {
        mathesis_core::model::SubmissionKind::Paper => "Research paper",
        mathesis_core::model::SubmissionKind::Note => "Short note",
        mathesis_core::model::SubmissionKind::Conjecture => "Conjecture",
    };
    let citation_pdf = if has_pdf {
        format!("<meta name=\"citation_pdf_url\" content=\"{pdf_url}\">")
    } else {
        String::new()
    };
    let pdf_link = if has_pdf {
        format!("<p><a href=\"{pdf_url}\">Download the PDF</a></p>")
    } else {
        String::new()
    };
    let body = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{title} · Mathesis</title><meta name=\"description\" content=\"{abstract_text}\"><meta name=\"citation_title\" content=\"{title}\">{authors}<meta name=\"citation_publication_date\" content=\"{date}\">{citation_pdf}<meta name=\"citation_fulltext_html_url\" content=\"{landing_url}\"><meta name=\"citation_technical_report_institution\" content=\"Omega Institute · Mathesis\"><link rel=\"canonical\" href=\"{landing_url}\"></head><body><main><article><p>Mathesis record <code>{record}</code> · {kind}</p><h1>{title}</h1><p class=\"authors\">{author_list}</p><p>Published {date} by Omega Institute · Mathesis.</p><h2>Abstract</h2><p>{abstract_text}</p>{pdf_link}<p><a href=\"{origin}/papers/{record}\">Open the interactive Mathesis page</a></p></article></main></body></html>"
    );
    let mut response = body.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    Ok(response)
}

async fn sitemap(State(app): AppState, headers: HeaderMap) -> ApiResult<Response> {
    let origin = request_origin(&headers);
    let mut records = Vec::new();
    let mut before = None;
    loop {
        let page = app
            .list_papers(Some(mathesis_core::app::MAX_PAGE), before)
            .await?;
        records.extend(page.items.into_iter().filter(|paper| paper.public));
        before = page.next_before;
        if before.is_none() {
            break;
        }
    }
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">",
    );
    xml.push_str(&format!(
        "<url><loc>{}</loc></url><url><loc>{}/papers</loc></url>",
        html_escape(&origin),
        html_escape(&origin)
    ));
    for paper in records {
        let record = html_escape(paper.record.as_str());
        let date = paper.accepted_at.format("%Y-%m-%d");
        xml.push_str(&format!(
            "<url><loc>{origin}/papers/{record}/landing.html</loc><lastmod>{date}</lastmod></url>",
            origin = html_escape(&origin),
            record = record,
        ));
    }
    xml.push_str("</urlset>");
    let mut response = xml.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/xml; charset=utf-8"),
    );
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=300"),
    );
    Ok(response)
}

fn request_origin(headers: &HeaderMap) -> String {
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| matches!(*value, "http" | "https"))
        .unwrap_or("https");
    let host = headers
        .get("x-forwarded-host")
        .or_else(|| headers.get(header::HOST))
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| {
            !value.is_empty()
                && value.chars().all(|character| {
                    character.is_ascii_alphanumeric()
                        || matches!(character, '.' | ':' | '-' | '[' | ']')
                })
        })
        .unwrap_or("localhost");
    format!("{scheme}://{host}")
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConjectureQuery {
    limit: Option<u32>,
    before: Option<String>,
    status: Option<String>,
}
async fn list_conjectures(
    State(app): AppState,
    Query(q): Query<ConjectureQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.list_conjectures_by_status(q.limit, q.before, q.status)
            .await?,
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LeanStatementBody {
    digest: String,
    confirm: bool,
    #[serde(default)]
    comment: String,
}
async fn respond_lean_statement(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    crate::auth::RequestAuthentication(authentication): crate::auth::RequestAuthentication,
    Path(id): Path<String>,
    Body(body): Body<LeanStatementBody>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.respond_lean_statement(
            &caller,
            authentication,
            &SubmissionId(id),
            &body.digest,
            body.confirm,
            body.comment,
        )
        .await?,
    ))
}

async fn review_markdown(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    let report = app.review_report(&caller, &SubmissionId(id)).await?;
    Ok((
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (header::CACHE_CONTROL, "private, no-store"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=mathesis-review.md",
            ),
        ],
        report.markdown(),
    ))
}

#[derive(Deserialize)]
struct EvidenceQuery {
    version: Option<u32>,
    path: String,
}
async fn evidence_file(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<SubmissionId>,
    Query(query): Query<EvidenceQuery>,
) -> ApiResult<impl IntoResponse> {
    let file = app
        .paper_evidence_file(&caller, &id, query.version, &query.path)
        .await?;
    let mut response = file_response(file, false);
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        HeaderValue::from_static("private, no-store"),
    );
    Ok(response)
}

async fn prepare_submission(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    request: Request,
) -> ApiResult<impl IntoResponse> {
    let (preparation, source) = if request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(';')
                .next()
                .is_some_and(|v| v.trim() == "application/json")
        }) {
        use base64::Engine;
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct EncodedPreparation {
            metadata: PrepareSubmission,
            filename: String,
            source_base64: String,
        }
        let Body(body) = Body::<EncodedPreparation>::from_request(request, &app).await?;
        if body.source_base64.len() > MAX_UPLOAD_BYTES * 4 / 3 {
            return Err(Problem(CoreError::invalid(
                "manuscript source exceeds 30 MiB",
            )));
        }
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&body.source_base64)
            .map_err(|_| Problem(CoreError::invalid("invalid source_base64")))?;
        (
            body.metadata,
            Upload {
                filename: body.filename,
                bytes,
            },
        )
    } else {
        let multipart = Multipart::from_request(request, &app)
            .await
            .map_err(|e| Problem(CoreError::invalid(e.to_string())))?;
        let form = read_form(multipart).await?;
        let metadata = form
            .metadata
            .ok_or_else(|| Problem(CoreError::invalid("attach preparation metadata")))?;
        let preparation: PrepareSubmission = serde_json::from_str(&metadata)
            .map_err(|e| Problem(CoreError::invalid(format!("metadata: {e}"))))?;
        (preparation, form.source.ok_or_else(missing_source)?)
    };
    Ok((
        StatusCode::CREATED,
        Json(app.prepare_submission(&caller, preparation, source).await?),
    ))
}

async fn submit_prepared(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Body(request): Body<SubmitPrepared>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.submit_prepared(&caller, &SubmissionId(id), request)
            .await?,
    ))
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ResearchProblemQuery {
    limit: Option<u32>,
    before: Option<String>,
}
async fn list_research_problems(
    State(app): AppState,
    Query(q): Query<ResearchProblemQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(app.list_research_problems(q.limit, q.before).await?))
}
async fn research_problem(
    State(app): AppState,
    Path(id): Path<String>,
    Query(q): Query<ResearchProblemQuery>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.research_problem_page(&id, q.before.as_deref()).await?,
    ))
}

async fn record_explanation(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
    Body(input): Body<mathesis_core::model::NewPaperExplanation>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.record_paper_explanation(&caller, &SubmissionId(id), input)
            .await?,
    ))
}

async fn request_explanation(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    Ok(Json(
        app.request_paper_explanation(&caller, &SubmissionId(id))
            .await?,
    ))
}
