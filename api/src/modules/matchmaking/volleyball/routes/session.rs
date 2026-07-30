use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    matchmaking::volleyball::{
        domain::session::SessionFilters,
        handler::session::use_cases::{AddRosterPlayersRequest, CreateSessionRequest},
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    let main_routes = Router::new()
        .route("/", post(create_session))
        .route("/list", post(list_sessions));

    let session_id_routes = Router::new().nest(
        "/{session_id}",
        Router::new()
            .route("/roster", post(add_players_to_roster))
            .route("/state", get(get_session_state))
            .route("/start", post(start_session))
            .route("/close", patch(close_session)),
    );

    Router::new().nest(
        "/session",
        Router::new().merge(main_routes).merge(session_id_routes),
    )
}

async fn create_session(
    state: State<AppState>,
    Json(request): Json<CreateSessionRequest>,
) -> HttpResult<impl IntoResponse> {
    let session = state
        .volleyball_state
        .session_handler
        .create_session(request)
        .await?;

    Ok(Json(session))
}

async fn list_sessions(
    state: State<AppState>,
    Json(filters): Json<SessionFilters>,
) -> HttpResult<impl IntoResponse> {
    let sessions = state
        .volleyball_state
        .session_handler
        .list_sessions(filters)
        .await?;

    Ok(Json(sessions))
}

async fn add_players_to_roster(
    state: State<AppState>,
    Path(session_id): Path<Uuid>,
    Json(request): Json<AddRosterPlayersRequest>,
) -> HttpResult<impl IntoResponse> {
    let players = state
        .volleyball_state
        .session_handler
        .add_players_to_roster(session_id, request)
        .await?;

    Ok(Json(players))
}

async fn get_session_state(
    state: State<AppState>,
    Path(session_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    let session_state = state
        .volleyball_state
        .session_handler
        .get_session_state(session_id)
        .await?;

    Ok(Json(session_state))
}

async fn start_session(
    state: State<AppState>,
    Path(session_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    let games = state
        .volleyball_state
        .session_handler
        .start_session(session_id)
        .await?;

    Ok(Json(games))
}

async fn close_session(
    state: State<AppState>,
    Path(session_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    let session = state
        .volleyball_state
        .session_handler
        .close_session(session_id)
        .await?;

    Ok(Json(session))
}
