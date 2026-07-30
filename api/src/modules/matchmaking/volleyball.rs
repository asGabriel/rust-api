use std::sync::Arc;

use axum::Router;

use crate::modules::{
    routes::AppState,
    matchmaking::volleyball::handler::{
        game::DynGameHandler, player::DynPlayerHandler, session::DynSessionHandler,
    },
};

pub mod domain;
pub mod handler;
pub mod repository;
pub mod routes;

pub struct VolleyballState {
    pub player_handler: Arc<DynPlayerHandler>,
    pub session_handler: Arc<DynSessionHandler>,
    pub game_handler: Arc<DynGameHandler>,
}

pub fn configure_service_routes() -> Router<AppState> {
    Router::new().nest(
        "/volleyball",
        Router::new()
            .merge(routes::player::configure_routes())
            .merge(routes::session::configure_routes())
            .merge(routes::game::configure_routes()),
    )
}
