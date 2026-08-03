use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use http_error::HttpResult;
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
            .route("/{id}", get(get_session).patch(update_session)),
    )
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
