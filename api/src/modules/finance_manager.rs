use std::sync::Arc;

use axum::Router;

use crate::modules::{
    finance_manager::handler::{
        debt::DynDebtHandler, financial_instrument::DynFinancialInstrumentHandler,
        income::DynIncomeHandler, payment::DynPaymentHandler, recurrence::DynRecurrenceHandler,
    },
    routes::AppState,
};

pub mod domain;
pub mod handler;
pub mod repository;
pub mod routes;

pub struct FinanceManagerState {
    pub income_handler: Arc<DynIncomeHandler>,
    pub payment_handler: Arc<DynPaymentHandler>,
    pub debt_handler: Arc<DynDebtHandler>,
    pub financial_instrument_handler: Arc<DynFinancialInstrumentHandler>,
    pub recurrence_handler: Arc<DynRecurrenceHandler>,
}

pub fn configure_service_routes() -> Router<AppState> {
    Router::new().nest(
        "/financeManager",
        Router::new()
            .merge(routes::payment::configure_routes())
            .merge(routes::debt::configure_routes())
            .merge(routes::financial_instrument::configure_routes())
            .merge(routes::recurrence::configure_routes())
            .merge(routes::income::configure_routes()),
    )
}
