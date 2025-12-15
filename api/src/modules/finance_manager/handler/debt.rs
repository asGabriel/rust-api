use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};

use crate::modules::finance_manager::{
    domain::{
        debt::{category::DebtCategory, Debt, DebtFilters},
        payment::Payment,
    },
    handler::{
        debt::use_cases::{CreateCategoryRequest, CreateDebtRequest},
        payment::use_cases::PaymentBasicData,
        pubsub::DynPubSubHandler,
    },
    repository::{
        account::DynAccountRepository,
        debt::{category::DynDebtCategoryRepository, DynDebtRepository},
        payment::DynPaymentRepository,
    },
};
use std::sync::Arc;

pub type DynDebtHandler = dyn DebtHandler + Send + Sync;

#[async_trait]
pub trait DebtHandler {
    async fn list_debts(&self, filters: &DebtFilters) -> HttpResult<Vec<Debt>>;
    async fn create_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt>;

    // DEBT_CATEGORY
    async fn create_debt_category(
        &self,
        request: CreateCategoryRequest,
    ) -> HttpResult<DebtCategory>;
    async fn list_debt_categories(&self) -> HttpResult<Vec<DebtCategory>>;
}

#[derive(Clone)]
pub struct DebtHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
    pub account_repository: Arc<DynAccountRepository>,
    pub payment_repository: Arc<DynPaymentRepository>,
    pub debt_category_repository: Arc<DynDebtCategoryRepository>,
    pub pubsub: Arc<DynPubSubHandler>,
}

#[async_trait]
impl DebtHandler for DebtHandlerImpl {
    async fn create_debt_category(
        &self,
        request: CreateCategoryRequest,
    ) -> HttpResult<DebtCategory> {
        let category = DebtCategory::new(request.name);
        self.debt_category_repository.insert(category).await
    }

    async fn list_debt_categories(&self) -> HttpResult<Vec<DebtCategory>> {
        self.debt_category_repository.list().await
    }

    async fn create_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt> {
        self.debt_category_repository
            .get_by_name(&request.category_name)
            .await?
            .or_not_found("category", &request.category_name)?;

        let debt = Debt::from_request(&request)?;
        let debt = self.debt_repository.insert(debt).await?;

        // TODO: dispatch payment create event
        if request.is_paid() {
            let account_id = request.account_id.ok_or_else(|| {
                Box::new(http_error::HttpError::bad_request(
                    "Account ID é obrigatório quando a despesa está paga",
                ))
            })?;

            let payment = Payment::new(
                &debt,
                &account_id,
                &PaymentBasicData {
                    amount: Some(*debt.total_amount()),
                    payment_date: *debt.due_date(),
                    force_settlement: false,
                },
            );

            let payment = self.payment_repository.insert(payment).await?;

            self.pubsub.publish_debt_updated_event(&payment).await?;
        }

        Ok(debt)
    }

    async fn list_debts(&self, filters: &DebtFilters) -> HttpResult<Vec<Debt>> {
        self.debt_repository.list(filters).await
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    use crate::modules::finance_manager::domain::debt::DebtStatus;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateDebtRequest {
        pub category_name: String,
        pub description: String,
        pub due_date: NaiveDate,
        pub total_amount: Decimal,
        pub paid_amount: Option<Decimal>,
        pub discount_amount: Option<Decimal>,
        pub status: Option<DebtStatus>,
        pub is_paid: bool,
        pub account_id: Option<uuid::Uuid>,
    }

    impl CreateDebtRequest {
        pub fn new(
            category_name: String,
            description: String,
            total_amount: Decimal,
            due_date: NaiveDate,
            is_paid: Option<bool>,
        ) -> Self {
            Self {
                category_name,
                description,
                total_amount,
                paid_amount: None,
                discount_amount: None,
                due_date,
                status: Some(DebtStatus::Unpaid),
                is_paid: is_paid.unwrap_or(false),
                account_id: None,
            }
        }

        pub fn is_paid(&self) -> bool {
            self.account_id.is_some()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateCategoryRequest {
        pub name: String,
    }
}
