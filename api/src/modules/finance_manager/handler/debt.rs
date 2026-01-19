use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::{
        debt::{
            installment::{Installment, InstallmentFilters},
            Debt, DebtFilters,
        },
        payment::Payment,
    },
    handler::{
        debt::use_cases::CreateDebtRequest, payment::use_cases::PaymentBasicData,
        pubsub::DynPubSubHandler,
    },
    repository::{
        account::DynAccountRepository,
        debt::{installment::DynInstallmentRepository, DynDebtRepository},
        payment::DynPaymentRepository,
        recurrence::DynRecurrenceRepository,
    },
};
use std::sync::Arc;

pub type DynDebtHandler = dyn DebtHandler + Send + Sync;

#[async_trait]
pub trait DebtHandler {
    async fn list_debts(&self, client_id: Uuid, filters: &DebtFilters) -> HttpResult<Vec<Debt>>;
    async fn register_new_debt(
        &self,
        client_id: Uuid,
        request: CreateDebtRequest,
    ) -> HttpResult<Debt>;

    async fn list_debt_installments(
        &self,
        filters: &InstallmentFilters,
    ) -> HttpResult<Vec<Installment>>;
}

#[derive(Clone)]
pub struct DebtHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
    pub installment_repository: Arc<DynInstallmentRepository>,
    pub account_repository: Arc<DynAccountRepository>,
    pub payment_repository: Arc<DynPaymentRepository>,
    pub recurrence_repository: Arc<DynRecurrenceRepository>,
    pub pubsub: Arc<DynPubSubHandler>,
}

impl DebtHandlerImpl {
    async fn create_debt(&self, client_id: Uuid, request: CreateDebtRequest) -> HttpResult<Debt> {
        let debt = Debt::new(
            client_id,
            request.description,
            request.total_amount,
            request.paid_amount,
            request.discount_amount,
            request.due_date,
            request.category,
            request.tags,
            request.installment_count,
        );
        let debt = self.debt_repository.insert(debt).await?;

        if let Some(installments) = debt.generate_installments()? {
            let _installments = self
                .installment_repository
                .insert_many(installments)
                .await?;
        }

        Ok(debt)
    }
}

#[async_trait]
impl DebtHandler for DebtHandlerImpl {
    async fn list_debt_installments(
        &self,
        filters: &InstallmentFilters,
    ) -> HttpResult<Vec<Installment>> {
        self.installment_repository.list(filters).await
    }

    async fn register_new_debt(
        &self,
        client_id: Uuid,
        request: CreateDebtRequest,
    ) -> HttpResult<Debt> {
        let mut debt = self.create_debt(client_id, request.clone()).await?;

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
                },
            );

            let payment = self.payment_repository.insert(payment).await?;

            debt = self.pubsub.process_debt_payment(debt, &payment).await?;
        }

        Ok(debt)
    }

    async fn list_debts(&self, client_id: Uuid, filters: &DebtFilters) -> HttpResult<Vec<Debt>> {
        let mut new_filters = DebtFilters::new(client_id);

        if let Some(statuses) = filters.statuses() {
            new_filters = new_filters.with_statuses(statuses.clone());
        }
        if let Some(start_date) = filters.start_date() {
            new_filters = new_filters.with_start_date(*start_date);
        }
        if let Some(end_date) = filters.end_date() {
            new_filters = new_filters.with_end_date(*end_date);
        }
        if let Some(category_names) = filters.category_names() {
            new_filters = new_filters.with_category_names(category_names.clone());
        }

        self.debt_repository.list(&new_filters).await
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::debt::{DebtCategory, DebtStatus};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateDebtRequest {
        pub category: Option<DebtCategory>,
        pub tags: Option<Vec<String>>,
        pub description: String,
        pub due_date: NaiveDate,
        pub total_amount: Decimal,
        pub paid_amount: Option<Decimal>,
        pub discount_amount: Option<Decimal>,
        pub status: Option<DebtStatus>,
        pub is_paid: bool,
        pub account_id: Option<Uuid>,
        pub installment_count: Option<i32>,
    }

    impl CreateDebtRequest {
        pub fn new(
            category: Option<DebtCategory>,
            tags: Option<Vec<String>>,
            description: String,
            total_amount: Decimal,
            due_date: NaiveDate,
            is_paid: Option<bool>,
            installment_count: Option<i32>,
        ) -> Self {
            Self {
                category,
                tags,
                description,
                total_amount,
                paid_amount: None,
                discount_amount: None,
                due_date,
                status: Some(DebtStatus::Unpaid),
                is_paid: is_paid.unwrap_or(false),
                account_id: None,
                installment_count,
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
