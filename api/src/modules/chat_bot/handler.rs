use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use telegram_api::domain::send_message::SendMessageRequest;

use crate::modules::{
    chat_bot::{
        domain::{
            debt::NewDebtData, formatter::ChatFormatter, payment::NewPaymentData, ChatCommand,
            ChatCommandType,
        },
        gateway::DynTelegramApiGateway,
    },
    finance_manager::{
        domain::debt::{DebtFilters, DebtStatus},
        handler::{
            account::DynAccountHandler,
            debt::{CreateDebtRequest, DynDebtHandler},
            payment::{
                use_cases::{
                    CreatePaymentRequest, PaymentBasicData, PaymentRequestFromIdentification,
                },
                DynPaymentHandler,
            },
        },
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
    pub account_handler: Arc<DynAccountHandler>,
    pub payment_handler: Arc<DynPaymentHandler>,
}

impl ChatBotHandlerImpl {
    pub async fn handle_list_debts(&self, chat_id: i64, filters: DebtFilters) -> HttpResult<()> {
        let result = self.debt_handler.list_debts(filters).await;

        let message = match result {
            Ok(debts) => ChatFormatter::format_list_for_chat(&debts),
            Err(e) => format!("❌ Erro ao listar débitos: {}", e.message),
        };

        self.send_message(chat_id, message).await?;
        Ok(())
    }

    async fn handle_new_debt(&self, request: NewDebtData, chat_id: i64) -> HttpResult<()> {
        let result = self
            .debt_handler
            .create_debt(CreateDebtRequest {
                account_identification: request.account_identification.clone(),
                description: request.description.clone(),
                total_amount: request.amount,
                paid_amount: None,
                discount_amount: Some(rust_decimal::Decimal::ZERO),
                due_date: request
                    .due_date
                    .unwrap_or_else(|| chrono::Utc::now().date_naive()),
                status: Some(DebtStatus::Unpaid),
                is_paid: request.is_paid(),
            })
            .await;

        let message = match result {
            Ok(debt) => format!(
                "✅ Despesa criada com sucesso! {}, {} - {}",
                debt.description(),
                debt.total_amount(),
                debt.due_date().format("%d/%m/%Y"),
            ),
            Err(e) => format!("❌ Erro ao criar despesa: {}", e.message),
        };

        self.send_message(chat_id, message).await?;
        Ok(())
    }

    async fn handle_list_accounts(&self, chat_id: i64) -> HttpResult<()> {
        let result = self.account_handler.list_accounts().await;

        let message = match result {
            Ok(accounts) => ChatFormatter::format_list_for_chat(&accounts),
            Err(e) => format!("❌ Erro ao listar contas: {}", e.message),
        };

        self.send_message(chat_id, message).await?;
        Ok(())
    }

    async fn handle_help(&self, chat_id: i64) -> HttpResult<()> {
        let message = ChatCommand::get_help_message();
        self.send_message(chat_id, message).await?;
        Ok(())
    }

    async fn handle_new_payment(&self, payment: NewPaymentData, chat_id: i64) -> HttpResult<()> {
        let result = self
            .payment_handler
            .create_payment(CreatePaymentRequest::PaymentRequestFromIdentification(
                PaymentRequestFromIdentification {
                    debt_identification: payment.debt_identification,
                    payment_basic_data: PaymentBasicData {
                        amount: payment.amount,
                        payment_date: payment
                            .payment_date
                            .unwrap_or(chrono::Utc::now().date_naive()),
                    },
                },
            ))
            .await;

        let message = match result {
            Ok(_) => "✅ Pagamento criado com sucesso!".to_string(),
            Err(e) => format!("❌ Erro ao criar pagamento: {}", e.message),
        };

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
            ChatCommandType::Help => {
                self.handle_help(chat_id).await?;
                Ok(())
            }
            ChatCommandType::Summary(filters) => {
                self.handle_list_debts(chat_id, filters.to_debt_filters())
                    .await?;
                Ok(())
            }
            ChatCommandType::ListAccounts => {
                self.handle_list_accounts(chat_id).await?;
                Ok(())
            }
            ChatCommandType::NewDebt(payload) => {
                self.handle_new_debt(payload, chat_id).await?;
                Ok(())
            }
            ChatCommandType::NewPayment(payment) => {
                self.handle_new_payment(payment, chat_id).await?;

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
