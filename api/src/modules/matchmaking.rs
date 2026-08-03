use axum::Router;

use crate::modules::routes::AppState;

pub mod routes;

pub fn configure_service_routes() -> Router<AppState> {
    Router::new().nest("/matchmaking", routes::configure_routes())
}
