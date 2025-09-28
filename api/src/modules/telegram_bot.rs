use std::sync::Arc;

use crate::modules::finance_manager::handler::payment::DynPaymentHandler;

pub mod gateway;
pub mod routes;

pub struct TelegramBotState {
    pub payment_handler: Arc<DynPaymentHandler>,
}
