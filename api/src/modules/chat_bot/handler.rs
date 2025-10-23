use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use telegram_api::domain::send_message::SendMessageRequest;

use crate::modules::{
    chat_bot::{
        domain::{ChatCommand, ChatCommandType},
        formatter::ChatFormatter,
        gateway::DynTelegramApiGateway,
    },
    finance_manager::{
        domain::debt::{Debt, DebtFilters},
        handler::debt::DynDebtHandler,
    },
};

pub type DynChatBotHandler = dyn ChatBotHandler + Send + Sync;

#[async_trait]
pub trait ChatBotHandler {
    async fn handle_command(&self, command: ChatCommand, chat_id: i64) -> HttpResult<()>;
}

pub struct ChatBotHandlerImpl {
    pub telegram_gateway: Arc<DynTelegramApiGateway>,
    pub debt_handler: Arc<DynDebtHandler>,
}

impl ChatBotHandlerImpl {
    pub async fn handle_list_debts(&self, chat_id: i64) -> HttpResult<()> {
        let debts = self.debt_handler.list_debts(DebtFilters::default()).await?;

        let message = Debt::format_list_for_chat(&debts);

        self.send_message(chat_id, message).await?;

        Ok(())
    }

    async fn send_message(&self, chat_id: i64, message: String) -> HttpResult<()> {
        self.telegram_gateway
            .send_message(SendMessageRequest {
                chat_id,
                text: message,
            })
            .await?;

        Ok(())
    }
}

#[async_trait]
impl ChatBotHandler for ChatBotHandlerImpl {
    async fn handle_command(&self, command: ChatCommand, chat_id: i64) -> HttpResult<()> {
        match command.command_type {
            ChatCommandType::Debts => {
                self.handle_list_debts(chat_id).await?;

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
