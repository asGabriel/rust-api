use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, patch},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    matchmaking::handler::player::use_cases::{CreatePlayerRequest, UpdatePlayerRequest},
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/players",
        Router::new()
            .route("/", get(list_players).post(create_player))
            .route("/{id}", patch(update_player)),
    )
}

async fn create_player(
    state: State<AppState>,
    Json(request): Json<CreatePlayerRequest>,
) -> HttpResult<impl IntoResponse> {
    let player = state
        .matchmaking_state
        .player_handler
        .create_player(request)
        .await?;

    Ok(Json(player))
}

async fn list_players(state: State<AppState>) -> HttpResult<impl IntoResponse> {
    let players = state
        .matchmaking_state
        .player_handler
        .list_players()
        .await?;

    Ok(Json(players))
}

async fn update_player(
    state: State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdatePlayerRequest>,
) -> HttpResult<impl IntoResponse> {
    let player = state
        .matchmaking_state
        .player_handler
        .update_player(id, request)
        .await?;

    Ok(Json(player))
}
