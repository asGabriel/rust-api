use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use rust_decimal::Decimal;
use telegram_api::domain::send_message::SendMessageRequest;
use uuid::Uuid;

use crate::modules::{
    chat_bot::{
        domain::{
            debt::NewDebtData,
            formatter::{ChatFormatter, ChatFormatterUtils},
            income::NewIncomeData,
            payment::NewPaymentData,
            summary::SummaryFilters,
            ChatCommand, ChatCommandType,
        },
        gateway::DynTelegramApiGateway,
    },
    finance_manager::{
        domain::debt::DebtStatus,
        handler::{
            debt::{use_cases::CreateDebtRequest, DynDebtHandler},
            financial_instrument::{
                use_cases::FinancialInstrumentListFilters, DynFinancialInstrumentHandler,
            },
            income::{use_cases::CreateIncomeRequest, DynIncomeHandler},
            payment::{
                use_cases::{
                    CreatePaymentRequest, PaymentBasicData, PaymentRequestFromIdentification,
                },
                DynPaymentHandler,
            },
        },
        repository::income::use_cases::IncomeListFilters,
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
    pub financial_instrument_handler: Arc<DynFinancialInstrumentHandler>,
    pub payment_handler: Arc<DynPaymentHandler>,
    pub income_handler: Arc<DynIncomeHandler>,
    pub client_id: Uuid,
}

impl ChatBotHandlerImpl {
    pub async fn handle_list_debts(&self, chat_id: i64, filters: SummaryFilters) -> HttpResult<()> {
        let (debt_filters, income_filters) = filters.to_filters();

        let result = self.debt_handler.list_debts(self.client_id, &debt_filters).await;

        let income_result = match self.income_handler.list_incomes(self.client_id, income_filters).await {
            Ok(incomes) => {
                let total_income: Decimal = incomes.iter().map(|i| *i.amount()).sum();
                format!(
                    "💰{} Total de receitas",
                    ChatFormatterUtils::format_currency(&total_income)
                )
            }
            Err(e) => format!("❌ Erro ao listar receitas: {}", e.message),
        };

        let mut message = match result {
            Ok(debts) => ChatFormatter::format_list_for_chat(&debts),
            Err(e) => format!("❌ Erro ao listar débitos: {}", e.message),
        };

        message = format!(
            "📊 Consulta de Débitos Mês: {}\n{}{}",
            debt_filters
                .start_date()
                .unwrap_or_else(|| chrono::Utc::now().date_naive())
                .format("%m/%Y"),
            income_result,
            message
        );

        self.send_message(chat_id, message).await?;
        Ok(())
    }

    async fn handle_new_debt(&self, request: NewDebtData, chat_id: i64) -> HttpResult<()> {
        let instrument_id = if let Some(instrument_identification) = &request.account_identification {
            let instrument = self
                .financial_instrument_handler
                .list_financial_instruments(
                    self.client_id,
                    FinancialInstrumentListFilters::new()
                        .with_identifications(vec![instrument_identification.clone()]),
                )
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| {
                    Box::new(http_error::HttpError::not_found(
                        "financial_instrument",
                        instrument_identification.clone(),
                    ))
                })?;

            Some(*instrument.id())
        } else {
            None
        };

        let result = self
            .debt_handler
            .register_new_debt(self.client_id, CreateDebtRequest {
                category: None,
                expense_type: None,
                tags: None,
                description: request.description.clone(),
                total_amount: request.amount,
                paid_amount: None,
                discount_amount: Some(rust_decimal::Decimal::ZERO),
                due_date: request.due_date,
                status: Some(DebtStatus::Unpaid),
                is_paid: request.is_paid(),
                financial_instrument_id: instrument_id,
                installment_count: request.installment_number,
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

    async fn handle_list_financial_instruments(&self, chat_id: i64) -> HttpResult<()> {
        let result = self
            .financial_instrument_handler
            .list_financial_instruments(self.client_id, FinancialInstrumentListFilters::default())
            .await;

        let message = match result {
            Ok(instruments) => ChatFormatter::format_list_for_chat(&instruments),
            Err(e) => format!("❌ Erro ao listar instrumentos financeiros: {}", e.message),
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
                    financial_instrument_identification: payment.account_identification,
                    reconcile: payment.settled,
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

    async fn handle_list_incomes(&self, chat_id: i64) -> HttpResult<()> {
        let result = self
            .income_handler
            .list_incomes(self.client_id, IncomeListFilters::default())
            .await;
        let message = match result {
            Ok(incomes) => ChatFormatter::format_list_for_chat(&incomes),
            Err(e) => format!("❌ Erro ao listar receitas: {}", e.message),
        };
        self.send_message(chat_id, message).await?;
        Ok(())
    }

    async fn handle_new_income(&self, income: NewIncomeData, chat_id: i64) -> HttpResult<()> {
        let result = self
            .income_handler
            .create_income(self.client_id, CreateIncomeRequest {
                financial_instrument_identification: income.account_identification,
                description: income.description,
                amount: income.amount,
                date_reference: income.date_reference,
            })
            .await;

        let message = match result {
            Ok(_) => "✅ Receita criada com sucesso!".to_string(),
            Err(e) => format!("❌ Erro ao criar receita: {}", e.message),
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
                self.handle_list_debts(chat_id, filters).await?;
                Ok(())
            }
            ChatCommandType::ListAccounts => {
                self.handle_list_financial_instruments(chat_id).await?;
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
            ChatCommandType::ListIncomes => {
                self.handle_list_incomes(chat_id).await?;
                Ok(())
            }
            ChatCommandType::NewIncome(income) => {
                self.handle_new_income(income, chat_id).await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
