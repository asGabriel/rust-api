use async_trait::async_trait;
use http_error::HttpResult;

use crate::modules::finance_manager::{
    domain::
        debt::{
            installment::{Installment, InstallmentFilters},
            Debt, DebtFilters,
        }
    ,
    handler::{
        debt::use_cases::CreateDebtRequest, pubsub::DynPubSubHandler,
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
    async fn list_debts(&self, filters: &DebtFilters) -> HttpResult<Vec<Debt>>;
    async fn register_new_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt>;

    async fn list_debt_installments(
        &self,
        filters: &InstallmentFilters,
    ) -> HttpResult<Vec<Installment>>;

    // async fn generate_debt_recurrences(&self) -> HttpResult<()>;
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
    async fn create_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt> {
        let debt = Debt::from(request);
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

    async fn register_new_debt(&self, request: CreateDebtRequest) -> HttpResult<Debt> {
        let debt = self.create_debt(request.clone()).await?;

        // if request.should_process_initial_payment() {
        //     let account_id = request.account_id.ok_or_else(|| {
        //         Box::new(http_error::HttpError::bad_request(
        //             "Account ID é obrigatório quando a despesa está paga",
        //         ))
        //     })?;

        //     let payment = Payment::new(
        //         &debt,
        //         &account_id,
        //         &PaymentBasicData {
        //             amount: Some(*debt.total_amount()),
        //             payment_date: *debt.due_date(),
        //         },
        //     );

        //     let payment = self.payment_repository.insert(payment).await?;

        //     debt = self.pubsub.process_debt_payment(debt, &payment).await?;
        // }

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

        /// Verify if the debt should be processed as a initial payment
        /// If the debt is paid and has an account ID, and doesn't have installments, it should be processed as a initial payment
        pub fn should_process_initial_payment(&self) -> bool {
            self.is_paid && self.account_id.is_some() && self.installment_count.is_none()
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateCategoryRequest {
        pub name: String,
    }
}
