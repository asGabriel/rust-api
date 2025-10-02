use async_trait::async_trait;
use chrono::NaiveDate;
use http_error::{ext::OptionHttpExt, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::debt::{Debt, DebtFilters, DebtStatus},
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
        let account = self
            .account_repository
            .get_by_id(request.account_id)
            .await?
            .or_not_found("account", request.account_id)?;

        let debt = self
            .debt_repository
            .insert(Debt::new(
                *account.id(),
                request.description,
                request.total_amount,
                request.paid_amount,
                request.discount_amount,
                request.due_date,
            ))
            .await?;

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
    account_id: Uuid,
    description: String,
    total_amount: Decimal,
    paid_amount: Option<Decimal>,
    discount_amount: Option<Decimal>,
    due_date: NaiveDate,
    status: Option<DebtStatus>,
}
