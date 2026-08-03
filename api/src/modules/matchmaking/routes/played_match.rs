use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    matchmaking::handler::played_match::use_cases::CreateMatchRequest, routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/matches",
        Router::new()
            .route("/", post(create_match))
            .route("/{game_day_id}", get(list_matches_by_game_day)),
    )
}

async fn create_match(
    state: State<AppState>,
    Json(request): Json<CreateMatchRequest>,
) -> HttpResult<impl IntoResponse> {
    let played_match = state
        .matchmaking_state
        .played_match_handler
        .create_match(request)
        .await?;

    Ok(Json(played_match))
}

async fn list_matches_by_game_day(
    state: State<AppState>,
    Path(game_day_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    let matches = state
        .matchmaking_state
        .played_match_handler
        .list_matches_by_game_day(game_day_id)
        .await?;

    Ok(Json(matches))
}
