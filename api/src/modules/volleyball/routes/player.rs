use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{patch, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    routes::AppState,
    volleyball::{
        domain::player::PlayerFilters,
        handler::player::use_cases::{CreatePlayerRequest, UpdatePlayerRequest},
    },
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/player",
        Router::new()
            .route("/", post(create_player))
            .route("/list", post(list_players))
            .route("/{player_id}", patch(update_player).delete(delete_player)),
    )
}

async fn create_player(
    state: State<AppState>,
    Json(request): Json<CreatePlayerRequest>,
) -> HttpResult<impl IntoResponse> {
    let player = state
        .volleyball_state
        .player_handler
        .create_player(request)
        .await?;

    Ok(Json(player))
}

async fn list_players(
    state: State<AppState>,
    Json(filters): Json<PlayerFilters>,
) -> HttpResult<impl IntoResponse> {
    let players = state
        .volleyball_state
        .player_handler
        .list_players(filters)
        .await?;

    Ok(Json(players))
}

async fn update_player(
    state: State<AppState>,
    Path(player_id): Path<Uuid>,
    Json(request): Json<UpdatePlayerRequest>,
) -> HttpResult<impl IntoResponse> {
    let player = state
        .volleyball_state
        .player_handler
        .update_player(player_id, request)
        .await?;

    Ok(Json(player))
}

async fn delete_player(
    state: State<AppState>,
    Path(player_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    state
        .volleyball_state
        .player_handler
        .delete_player(player_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
