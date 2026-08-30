use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{delete, get, patch, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{
    matchmaking::handler::team::use_cases::{CreateTeamRequest, UpdateTeamRequest},
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/teams",
        Router::new()
            .route("/", post(create_team))
            .route("/{session_id}", get(list_teams_by_session))
            .route("/{team_id}/players", patch(update_team))
            .route("/{team_id}", delete(discard_draft)),
    )
}

async fn create_team(
    state: State<AppState>,
    Json(request): Json<CreateTeamRequest>,
) -> HttpResult<impl IntoResponse> {
    let team = state
        .matchmaking_state
        .team_handler
        .create_team(request)
        .await?;

    Ok(Json(team))
}

async fn list_teams_by_session(
    state: State<AppState>,
    Path(session_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    let teams = state
        .matchmaking_state
        .team_handler
        .list_teams_by_session(session_id)
        .await?;

    Ok(Json(teams))
}

async fn update_team(
    state: State<AppState>,
    Path(team_id): Path<Uuid>,
    Json(request): Json<UpdateTeamRequest>,
) -> HttpResult<impl IntoResponse> {
    let team = state
        .matchmaking_state
        .team_handler
        .update_team(team_id, request)
        .await?;

    Ok(Json(team))
}

async fn discard_draft(
    state: State<AppState>,
    Path(team_id): Path<Uuid>,
) -> HttpResult<impl IntoResponse> {
    state
        .matchmaking_state
        .team_handler
        .discard_draft(team_id)
        .await?;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
