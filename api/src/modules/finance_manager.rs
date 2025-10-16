use std::sync::Arc;

use axum::Router;

use crate::modules::{
    finance_manager::handler::{
        account::DynAccountHandler, debt::DynDebtHandler, payment::DynPaymentHandler,
    },
    AppState,
};

pub mod domain;
pub mod handler;
pub mod repository;
pub mod routes;

pub struct FinanceManagerState {
    pub payment_handler: Arc<DynPaymentHandler>,
    pub debt_handler: Arc<DynDebtHandler>,
    pub account_handler: Arc<DynAccountHandler>,
}

pub fn configure_service_routes() -> Router<AppState> {
    Router::new().nest(
        "/financeManager",
        Router::new()
            .merge(routes::payment::configure_routes())
            .merge(routes::debt::configure_routes())
            .merge(routes::account::configure_routes()),
    )
}
