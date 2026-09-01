use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use http_error::HttpResult;
use serde::Deserialize;
use uuid::Uuid;

use crate::modules::{
    matchmaking::handler::session::use_cases::{CreateSessionRequest, UpdateSessionRequest},
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/sessions",
        Router::new()
            .route("/", get(list_sessions).post(create_session))
            .route("/{id}", get(get_session).patch(update_session))
            .route(
                "/{id}/check-in/{player_id}",
                post(check_in).delete(check_out),
            )
            .route("/{id}/queue", get(list_queue))
            .route("/{id}/queue/seed", post(seed_queue))
            .route("/{id}/queue/fill", post(fill_idle_courts))
            .route("/{id}/queue/{player_id}", patch(set_pin)),
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetPinRequest {
    pinned: bool,
}

async fn create_session(
    state: State<AppState>,
    Json(request): Json<CreateSessionRequest>,
) -> HttpResult<impl IntoResponse> {
    let session = state
        .matchmaking_state
        .session_handler
        .create_session(request)
        .await?;

    Ok(Json(session))
}

async fn list_sessions(state: State<AppState>) -> HttpResult<impl IntoResponse> {
    let sessions = state
        .matchmaking_state
        .session_handler
        .list_sessions()
        .await?;

    Ok(Json(sessions))
}

async fn get_session(
    state: State<AppState>,
    Path(id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    let session = state
        .matchmaking_state
        .session_handler
        .get_session(id)
        .await?;

    Ok(Json(session))
}

async fn update_session(
    state: State<AppState>,
    Path(id): Path<Uuid>,
    Json(request): Json<UpdateSessionRequest>,
) -> HttpResult<impl IntoResponse> {
    let session = state
        .matchmaking_state
        .session_handler
        .update_session(id, request)
        .await?;

    Ok(Json(session))
}

async fn check_in(
    state: State<AppState>,
    Path((id, player_id)): Path<(Uuid, Uuid)>,
) -> HttpResult<impl IntoResponse> {
    let session = state
        .matchmaking_state
        .session_handler
        .check_in(id, player_id)
        .await?;

    Ok(Json(session))
}

async fn check_out(
    state: State<AppState>,
    Path((id, player_id)): Path<(Uuid, Uuid)>,
) -> HttpResult<impl IntoResponse> {
    let session = state
        .matchmaking_state
        .session_handler
        .check_out(id, player_id)
        .await?;

    Ok(Json(session))
}

async fn list_queue(state: State<AppState>, Path(id): Path<Uuid>) -> HttpResult<impl IntoResponse> {
    let queue = state
        .matchmaking_state
        .session_handler
        .list_queue(id)
        .await?;

    Ok(Json(queue))
}

async fn seed_queue(state: State<AppState>, Path(id): Path<Uuid>) -> HttpResult<impl IntoResponse> {
    let drafts = state.matchmaking_state.team_handler.seed_queue(id).await?;

    Ok(Json(drafts))
}

async fn fill_idle_courts(
    state: State<AppState>,
    Path(id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    let rotation = state
        .matchmaking_state
        .team_handler
        .refresh_idle_courts(id)
        .await?;

    Ok(Json(rotation.courts))
}

async fn set_pin(
    state: State<AppState>,
    Path((id, player_id)): Path<(Uuid, Uuid)>,
    Json(request): Json<SetPinRequest>,
) -> HttpResult<impl IntoResponse> {
    let entry = state
        .matchmaking_state
        .session_handler
        .set_pin(id, player_id, request.pinned)
        .await?;

    Ok(Json(entry))
}
