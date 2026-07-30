use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{patch, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    matchmaking::volleyball::{
        domain::game::GameFilters, handler::game::use_cases::RecordGameResultRequest,
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/game",
        Router::new()
            .route("/list", post(list_games))
            .route("/{game_id}/result", patch(record_result)),
    )
}

async fn list_games(
    state: State<AppState>,
    Json(filters): Json<GameFilters>,
) -> HttpResult<impl IntoResponse> {
    let games = state
        .volleyball_state
        .game_handler
        .list_games(filters)
        .await?;

    Ok(Json(games))
}

async fn record_result(
    state: State<AppState>,
    Path(game_id): Path<Uuid>,
    Json(request): Json<RecordGameResultRequest>,
) -> HttpResult<impl IntoResponse> {
    let outcome = state
        .volleyball_state
        .game_handler
        .record_result(game_id, request)
        .await?;

    Ok(Json(outcome))
}
