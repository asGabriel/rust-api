use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{matchmaking::handler::matches::use_cases::CreateMatchRequest, routes::AppState};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/matches",
        Router::new()
            .route("/", post(create_match))
            .route("/{session_id}", get(list_matches_by_session)),
    )
}

async fn create_match(
    state: State<AppState>,
    Json(request): Json<CreateMatchRequest>,
) -> HttpResult<impl IntoResponse> {
    let match_ = state
        .matchmaking_state
        .match_handler
        .create_match(request)
        .await?;

    Ok(Json(match_))
}

async fn list_matches_by_session(
    state: State<AppState>,
    Path(session_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    let matches = state
        .matchmaking_state
        .match_handler
        .list_matches_by_session(session_id)
        .await?;

    Ok(Json(matches))
}
