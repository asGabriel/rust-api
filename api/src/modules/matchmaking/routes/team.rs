use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::{matchmaking::handler::team::use_cases::CreateTeamRequest, routes::AppState};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/teams",
        Router::new()
            .route("/", post(create_team))
            .route("/{session_id}", get(list_teams_by_session)),
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
