use std::sync::Arc;

use axum::Router;

use crate::modules::{
    matchmaking::handler::{
        played_match::DynPlayedMatchHandler, player::DynPlayerHandler, session::DynSessionHandler,
        team::DynTeamHandler,
    },
    routes::AppState,
};

pub mod domain;
pub mod handler;
pub mod repository;
pub mod routes;

pub struct MatchmakingState {
    pub player_handler: Arc<DynPlayerHandler>,
    pub session_handler: Arc<DynSessionHandler>,
    pub team_handler: Arc<DynTeamHandler>,
    pub played_match_handler: Arc<DynPlayedMatchHandler>,
}

pub fn configure_service_routes() -> Router<AppState> {
    Router::new().nest(
        "/matchmaking",
        Router::new()
            .merge(routes::configure_routes())
            .merge(routes::player::configure_routes())
            .merge(routes::session::configure_routes())
            .merge(routes::team::configure_routes())
            .merge(routes::played_match::configure_routes()),
    )
}
