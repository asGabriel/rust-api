use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use http_error::{ext::OptionHttpExt, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::debt::{generator::DebtGenerator, Debt, DebtFilters, DebtStatus},
    repository::{account::DynAccountRepository, debt::DynDebtRepository},
};
use std::sync::Arc;

pub type DynDebtHandler = dyn DebtHandler + Send + Sync;

#[async_trait]
pub trait DebtHandler {
    async fn list_debts(&self, filters: DebtFilters) -> HttpResult<Vec<Debt>>;
    async fn create_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt>;
}

pub struct DebtHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
    pub account_repository: Arc<DynAccountRepository>,
}

#[async_trait]
impl DebtHandler for DebtHandlerImpl {
    async fn create_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt> {
        let _account = self
            .account_repository
            .get_by_id(request.account_id)
            .await?
            .or_not_found("account", request.account_id)?;

        let debt_generator = DebtGenerator { request };

        let mut debt = debt_generator.generate_debt_from_request();

        if debt_generator.is_paid() {
            let payment = debt_generator.paid(&debt);
            debt.paid(payment);
        }

        let debt = self.debt_repository.insert(debt).await?;

        Ok(debt)
    }

    async fn list_debts(&self, filters: DebtFilters) -> HttpResult<Vec<Debt>> {
        self.debt_repository.list(filters).await
    }
}

// Use cases

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDebtRequest {
    pub account_id: Uuid,
    pub description: String,
    pub total_amount: Decimal,
    pub paid_amount: Option<Decimal>,
    pub discount_amount: Option<Decimal>,
    pub due_date: NaiveDate,
    pub status: Option<DebtStatus>,
    #[serde(flatten)]
    pub configuration: DebtConfiguration,
}

impl CreateDebtRequest {
    pub fn new(account_id: Uuid, description: String, total_amount: Decimal, due_date: Option<NaiveDate>) -> Self {
        Self {
            account_id,
            description,
            total_amount,
            paid_amount: None,
            discount_amount: None,
            due_date: due_date.unwrap_or(Utc::now().date_naive()),
            status: Some(DebtStatus::Unpaid),
            configuration: DebtConfiguration {
                is_paid: None,
                installments: None,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebtConfiguration {
    #[serde(default)]
    pub is_paid: Option<bool>,
    #[serde(default)]
    pub installments: Option<u8>,
}
