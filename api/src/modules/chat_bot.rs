use std::sync::Arc;

use crate::modules::finance_manager::handler::payment::DynPaymentHandler;

pub mod domain;
pub mod gateway;
pub mod handler;
pub mod routes;

pub struct ChatBotState {
    pub payment_handler: Arc<DynPaymentHandler>,
}
