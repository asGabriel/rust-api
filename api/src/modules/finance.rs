use std::sync::Arc;

use axum::Router;

use crate::modules::{finance::handler::debt::DynDebtHandler, routes::AppState};

pub mod domain;
pub mod handler;
pub mod repository;
pub mod routes;

pub struct FinanceState {
    pub debt_handler: Arc<DynDebtHandler>,
}

pub fn configure_service_routes() -> Router<AppState> {
    Router::new().nest(
        "/finance",
        Router::new().merge(routes::debt::configure_routes()),
    )
}
