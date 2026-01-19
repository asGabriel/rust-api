use std::sync::Arc;

use axum::Router;

use crate::modules::{auth::handler::DynAuthHandler, routes::AppState};

pub mod domain;
pub mod handler;
pub mod repository;
pub mod routes;

pub struct AuthState {
    pub auth_handler: Arc<DynAuthHandler>,
}

pub fn configure_service_routes() -> Router<AppState> {
    Router::new().nest("/auth", routes::configure_routes())
}
