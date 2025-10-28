use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use http_error::{ext::OptionHttpExt, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::modules::finance_manager::{
    domain::{
        debt::{generator::DebtGenerator, Debt, DebtFilters, DebtStatus},
        payment::Payment,
    },
    repository::{account::DynAccountRepository, debt::DynDebtRepository},
};
use std::sync::Arc;

pub type DynDebtHandler = dyn DebtHandler + Send + Sync;

#[async_trait]
pub trait DebtHandler {
    async fn list_debts(&self, filters: DebtFilters) -> HttpResult<Vec<Debt>>;
    async fn create_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt>;
    /// Handles the payment created event by updating the debt
    async fn payment_created_event(&self, payment: &Payment) -> HttpResult<()>;
}

#[derive(Clone)]
pub struct DebtHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
    pub account_repository: Arc<DynAccountRepository>,
}

#[async_trait]
impl DebtHandler for DebtHandlerImpl {
    async fn payment_created_event(&self, payment: &Payment) -> HttpResult<()> {
        let mut debt = self
            .debt_repository
            .get_by_id(payment.debt_id())
            .await?
            .or_not_found("debt", &payment.debt_id().to_string())?;

        debt.payment_created(&payment);
        self.debt_repository.update(debt).await?;

        Ok(())
    }

    async fn create_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt> {
        let account = self
            .account_repository
            .get_by_identification(&request.account_identification)
            .await?
            .or_not_found("account", &request.account_identification)?;

        let debt = DebtGenerator::generate_debt_from_request(request, *account.id());

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
    pub account_identification: String,
    pub description: String,
    pub total_amount: Decimal,
    pub paid_amount: Option<Decimal>,
    pub discount_amount: Option<Decimal>,
    pub due_date: NaiveDate,
    pub status: Option<DebtStatus>,
    pub is_paid: bool,
}

impl CreateDebtRequest {
    pub fn new(
        account_identification: String,
        description: String,
        total_amount: Decimal,
        due_date: Option<NaiveDate>,
        is_paid: Option<bool>,
    ) -> Self {
        Self {
            account_identification,
            description,
            total_amount,
            paid_amount: None,
            discount_amount: None,
            due_date: due_date.unwrap_or(Utc::now().date_naive()),
            status: Some(DebtStatus::Unpaid),
            is_paid: is_paid.unwrap_or(false),
        }
    }
}
