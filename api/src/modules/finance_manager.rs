use std::sync::Arc;

use crate::modules::finance_manager::handler::payment::DynPaymentHandler;

pub mod domain;
pub mod handler;
pub mod repository;
pub mod routes;

pub struct FinanceManagerState {
    pub payment_handler: Arc<DynPaymentHandler>,
}