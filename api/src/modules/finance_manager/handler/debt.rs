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
    async fn list_debts(&self, filters: DebtFilters) -> HttpResult<Vec<Debt>>;
    async fn create_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt>;

    // DEBT-CATEGORY
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
        let account = self
            .account_repository
            .get_by_identification(&request.account_identification)
            .await?
            .or_not_found("account", &request.account_identification)?;

        self.debt_category_repository
            .get_by_name(&request.category_name)
            .await?
            .or_not_found("category", &request.category_name)?;

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
pub mod use_cases {
    use chrono::{NaiveDate, Utc};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    use crate::modules::finance_manager::domain::debt::DebtStatus;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateDebtRequest {
        pub account_identification: String,
        pub category_name: String,
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
            category_name: String,
            description: String,
            total_amount: Decimal,
            due_date: Option<NaiveDate>,
            is_paid: Option<bool>,
        ) -> Self {
            Self {
                account_identification,
                category_name,
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

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateCategoryRequest {
        pub name: String,
    }
}
