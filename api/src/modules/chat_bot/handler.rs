use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;

use crate::modules::{
    chat_bot::domain::{ChatCommand, ChatCommandType},
    finance_manager::handler::debt::DynDebtHandler,
};

#[async_trait]
pub trait WebhookBotHandler {
    async fn handle_command(&self, command: ChatCommand) -> HttpResult<()>;
}

pub struct WebhookBotHandlerImpl {
    pub debt_handler: Arc<DynDebtHandler>,
}

#[async_trait]
impl WebhookBotHandler for WebhookBotHandlerImpl {
    async fn handle_command(&self, command: ChatCommand) -> HttpResult<()> {
        match command.command_type {
            ChatCommandType::Debts => Ok(()),
            _ => Ok(()),
        }
    }
}
