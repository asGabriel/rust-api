use std::sync::Arc;

use crate::modules::{
    chat_bot::handler::DynChatBotHandler, finance_manager::handler::payment::DynPaymentHandler,
};

use self::gateway::DynTelegramApiGateway;

pub mod domain;
pub mod gateway;
pub mod handler;
pub mod routes;

pub struct ChatBotState {
    pub chat_bot_handler: Arc<DynChatBotHandler>,
    pub payment_handler: Arc<DynPaymentHandler>,
    pub telegram_gateway: Arc<DynTelegramApiGateway>,
}
