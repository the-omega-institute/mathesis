use super::*;
use mathesis_core::model::{REQUEST_REPOSITORY, SharedRequestRank, SharedRequestsSnapshot};

pub(super) fn routes() -> Router<Arc<App>> {
    Router::new()
        .route("/formalization-requests", get(requests))
        .route("/formalization-requests/leaderboard", get(leaderboard))
        .route(
            "/staff/formalization-requests/snapshot",
            axum::routing::put(import_snapshot),
        )
}

async fn requests(State(app): AppState) -> ApiResult<Json<SharedRequestsSnapshot>> {
    Ok(Json(app.shared_formalization_requests().await?))
}

async fn import_snapshot(
    State(app): AppState,
    AuthenticatedCaller(caller): AuthenticatedCaller,
    Body(snapshot): Body<SharedRequestsSnapshot>,
) -> ApiResult<Json<SharedRequestsSnapshot>> {
    if !caller.has(mathesis_core::model::Role::Admin) {
        return Err(mathesis_core::CoreError::forbidden("administrator required").into());
    }
    let previous = app.shared_formalization_requests().await?;
    app.replace_shared_formalization_requests(&snapshot, previous.revision)
        .await?;
    Ok(Json(snapshot))
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BoardQuery {
    period: Option<String>,
}

#[derive(Serialize)]
struct Board {
    repository: &'static str,
    observed_at: Option<String>,
    rows: Vec<SharedRequestRank>,
}

async fn leaderboard(
    State(app): AppState,
    Query(query): Query<BoardQuery>,
) -> ApiResult<Json<Board>> {
    if query
        .period
        .as_deref()
        .is_some_and(|p| !["all", "month"].contains(&p))
    {
        return Err(mathesis_core::CoreError::invalid("invalid request leaderboard period").into());
    }
    let snapshot = app.shared_formalization_requests().await?;
    Ok(Json(Board {
        repository: REQUEST_REPOSITORY,
        observed_at: snapshot.observed_at.map(|at| at.to_rfc3339()),
        rows: snapshot.leaderboard(query.period.as_deref() == Some("month")),
    }))
}
