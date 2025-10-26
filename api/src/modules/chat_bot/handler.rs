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
        domain::debt::DebtFilters,
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
    pub async fn handle_list_debts(&self, chat_id: i64) -> HttpResult<()> {
        let debts = self.debt_handler.list_debts(DebtFilters::default()).await?;

        let message = ChatFormatter::format_list_for_chat(&debts);

        self.send_message(chat_id, message).await?;

        Ok(())
    }

    async fn handle_new_debt(&self, debt: NewDebtData, chat_id: i64) -> HttpResult<()> {
        self.debt_handler
            .create_debt(CreateDebtRequest {
                account_identification: debt.account_identification,
                description: debt.description,
                total_amount: debt.amount,
                paid_amount: Some(rust_decimal::Decimal::ZERO),
                discount_amount: Some(rust_decimal::Decimal::ZERO),
                due_date: chrono::Utc::now().date_naive(),
                status: Some(crate::modules::finance_manager::domain::debt::DebtStatus::Unpaid),
            })
            .await?;

        let message = format!("Debt created successfully.");
        self.send_message(chat_id, message).await?;

        Ok(())
    }

    async fn handle_list_accounts(&self, chat_id: i64) -> HttpResult<()> {
        let accounts = self.account_handler.list_accounts().await?;

        let message = ChatFormatter::format_list_for_chat(&accounts);

        self.send_message(chat_id, message).await?;

        Ok(())
    }

    async fn handle_new_payment(&self, payment: NewPaymentData, chat_id: i64) -> HttpResult<()> {
        self.payment_handler
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
            .await?;

        let message = format!("Payment created successfully.");
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
            ChatCommandType::Summary => {
                self.handle_list_debts(chat_id).await?;

                Ok(())
            }
            ChatCommandType::ListAccounts => {
                self.handle_list_accounts(chat_id).await?;

                Ok(())
            }
            ChatCommandType::NewDebt(debt) => {
                if let Err(e) = self.handle_new_debt(debt, chat_id).await {
                    self.telegram_gateway
                        .send_message(SendMessageRequest {
                            chat_id,
                            text: format!("Erro ao criar débito: {}", e),
                        })
                        .await?;
                }

                Ok(())
            }
            ChatCommandType::NewPayment(payment) => {
                if let Err(e) = self.handle_new_payment(payment, chat_id).await {
                    self.telegram_gateway
                        .send_message(SendMessageRequest {
                            chat_id,
                            text: format!("Erro ao criar pagamento: {}", e),
                        })
                        .await?;
                }

                Ok(())
            }
            _ => Ok(()),
        }
    }
}
