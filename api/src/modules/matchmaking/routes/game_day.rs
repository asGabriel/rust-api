use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    matchmaking::handler::game_day::use_cases::{CreateGameDayRequest, UpdateGameDayRequest},
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/gameDays",
        Router::new()
            .route("/", get(list_game_days).post(create_game_day))
            .route("/{id}", get(get_game_day).patch(update_game_day)),
    )
}

async fn create_game_day(
    state: State<AppState>,
    Json(request): Json<CreateGameDayRequest>,
) -> HttpResult<impl IntoResponse> {
    let game_day = state
        .matchmaking_state
        .game_day_handler
        .create_game_day(request)
        .await?;

    Ok(Json(game_day))
}

async fn list_game_days(state: State<AppState>) -> HttpResult<impl IntoResponse> {
    let game_days = state
        .matchmaking_state
        .game_day_handler
        .list_game_days()
        .await?;

    Ok(Json(game_days))
}

async fn get_game_day(
    state: State<AppState>,
    Path(id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    let game_day = state
        .matchmaking_state
        .game_day_handler
        .get_game_day(id)
        .await?;

    Ok(Json(game_day))
}

async fn update_game_day(
    state: State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateGameDayRequest>,
) -> HttpResult<impl IntoResponse> {
    let game_day = state
        .matchmaking_state
        .game_day_handler
        .update_game_day(id, request)
        .await?;

    Ok(Json(game_day))
}
