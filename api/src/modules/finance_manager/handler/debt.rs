use async_trait::async_trait;
use chrono::{NaiveDate, Utc};
use http_error::{ext::OptionHttpExt, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::modules::finance_manager::{
    domain::{
        debt::{Debt, DebtFilters, DebtStatus},
        payment::Payment,
    },
    handler::{payment::use_cases::PaymentBasicData, pubsub::DynPubSubHandler},
    repository::{
        account::DynAccountRepository, debt::DynDebtRepository, payment::DynPaymentRepository,
    },
};
use std::sync::Arc;

pub type DynDebtHandler = dyn DebtHandler + Send + Sync;

#[async_trait]
pub trait DebtHandler {
    async fn list_debts(&self, filters: DebtFilters) -> HttpResult<Vec<Debt>>;
    async fn create_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt>;
}

#[derive(Clone)]
pub struct DebtHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
    pub account_repository: Arc<DynAccountRepository>,
    pub payment_repository: Arc<DynPaymentRepository>,
    pub pubsub: Arc<DynPubSubHandler>,
}

#[async_trait]
impl DebtHandler for DebtHandlerImpl {
    async fn create_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt> {
        let account = self
            .account_repository
            .get_by_identification(&request.account_identification)
            .await?
            .or_not_found("account", &request.account_identification)?;

        let debt = Debt::from_request(&request, *account.id());

        let debt = self.debt_repository.insert(debt).await?;

        // TODO: dispatch payment create event
        if request.is_paid {
            let payment = Payment::new(
                &debt,
                &PaymentBasicData {
                    amount: Some(*debt.total_amount()),
                    payment_date: request.due_date,
                },
            );

            let payment = self.payment_repository.insert(payment).await?;

            self.pubsub.publish_debt_updated_event(&payment).await?;
        }

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
