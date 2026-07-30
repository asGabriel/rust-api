use axum::Router;

use crate::modules::routes::AppState;

pub mod volleyball;

/// Container for per-sport matchmaking modules. Each sport (volleyball
/// today, others later) owns its own domain/handler/repository/routes and
/// DB schema; this just nests them all under `/matchmaking/<sport>`.
pub fn configure_service_routes() -> Router<AppState> {
    Router::new().nest("/matchmaking", volleyball::configure_service_routes())
}
