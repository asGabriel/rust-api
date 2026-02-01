use async_trait::async_trait;
use chrono::{Datelike, Utc};
use http_error::{ext::OptionHttpExt, HttpResult};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::debt::{
        installment::{Installment, InstallmentFilters},
        recurrence::{Recurrence, RecurrenceFilters},
        Debt, DebtFilters,
    },
    handler::debt::use_cases::{
        CreateDebtRequest, CreateRecurrenceRequest, UpdateDebtRequest, UpdateRecurrenceRequest,
    },
    repository::{
        debt::{installment::DynInstallmentRepository, DynDebtRepository},
        financial_instrument::DynFinancialInstrumentRepository,
        recurrence::DynRecurrenceRepository,
    },
};
use std::sync::Arc;

pub type DynDebtHandler = dyn DebtHandler + Send + Sync;

#[async_trait]
pub trait DebtHandler {
    async fn update_debt(
        &self,
        client_id: Uuid,
        debt_id: Uuid,
        request: UpdateDebtRequest,
    ) -> HttpResult<Debt>;

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

    async fn generate_current_recurrences(&self) -> HttpResult<()>;

    async fn create_debt_recurrence(
        &self,
        client_id: Uuid,
        request: CreateRecurrenceRequest,
    ) -> HttpResult<Recurrence>;

    async fn list_debt_recurrences(
        &self,
        client_id: Uuid,
        filters: &RecurrenceFilters,
    ) -> HttpResult<Vec<Recurrence>>;

    async fn update_debt_recurrence(
        &self,
        client_id: Uuid,
        recurrence_id: Uuid,
        request: UpdateRecurrenceRequest,
    ) -> HttpResult<Recurrence>;
}

#[derive(Clone)]
pub struct DebtHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
    pub financial_instrument_repository: Arc<DynFinancialInstrumentRepository>,
    pub installment_repository: Arc<DynInstallmentRepository>,
    pub recurrence_repository: Arc<DynRecurrenceRepository>,
}

impl DebtHandlerImpl {
    async fn create_debt(&self, client_id: Uuid, request: CreateDebtRequest) -> HttpResult<Debt> {
        request.validate()?;

        let mut debt = Debt::new(
            client_id,
            request.description,
            request.total_amount,
            request.paid_amount,
            request.discount_amount,
            request.due_date,
            request.category,
            request.expense_type,
            request.tags,
            request.installment_count,
        );

        let installments = self
            .process_installments(&mut debt, request.financial_instrument_id)
            .await?;

        let debt = self.debt_repository.insert(debt).await?;

        if let Some(installments) = installments {
            self.installment_repository
                .insert_many(installments)
                .await?;
        }

        Ok(debt)
    }

    /// Processes installments for a debt if applicable.
    /// Returns None if no installments, or the generated installments.
    /// Also updates the debt's due_date to the last installment date.
    async fn process_installments(
        &self,
        debt: &mut Debt,
        financial_instrument_id: Option<Uuid>,
    ) -> HttpResult<Option<Vec<Installment>>> {
        if !debt.has_installments() {
            return Ok(None);
        }

        let instrument_id = financial_instrument_id.ok_or_else(|| {
            Box::new(http_error::HttpError::bad_request(
                "Financial instrument is required for installment debts",
            ))
        })?;

        let instrument = self
            .financial_instrument_repository
            .get_by_id(instrument_id)
            .await?
            .ok_or_else(|| {
                Box::new(http_error::HttpError::not_found(
                    "financial_instrument",
                    instrument_id,
                ))
            })?;

        let due_day = instrument.configuration().default_due_date.ok_or_else(|| {
            Box::new(http_error::HttpError::bad_request(
                "Financial instrument must have a configured due date for installment debts",
            ))
        })?;

        let installments = debt.generate_installments(due_day)?;
        Ok(Some(installments))
    }
}

#[async_trait]
impl DebtHandler for DebtHandlerImpl {
    async fn generate_current_recurrences(&self) -> HttpResult<()> {
        let today = Utc::now().date_naive();
        let current_year = today.year();
        let current_month = today.month();

        let recurrences = self
            .recurrence_repository
            .list(&RecurrenceFilters::new().with_active(true))
            .await?;

        for mut recurrence in recurrences {
            // Skip if already executed this month
            if recurrence.was_executed_in_month(current_year, current_month) {
                continue;
            }

            // Skip if outside the valid date range
            if !recurrence.is_within_date_range(today) {
                continue;
            }

            let debt = recurrence.generate_debt_for_month(current_year, current_month);
            let saved_debt = self.debt_repository.insert(debt).await?;

            recurrence.add_execution_log(today, *saved_debt.id());
            self.recurrence_repository.update(recurrence).await?;
        }

        Ok(())
    }

    async fn list_debt_recurrences(
        &self,
        client_id: Uuid,
        filters: &RecurrenceFilters,
    ) -> HttpResult<Vec<Recurrence>> {
        let filters = filters.clone().with_client_id(client_id);
        self.recurrence_repository.list(&filters).await
    }

    async fn update_debt_recurrence(
        &self,
        client_id: Uuid,
        recurrence_id: Uuid,
        request: UpdateRecurrenceRequest,
    ) -> HttpResult<Recurrence> {
        let mut recurrence = self
            .recurrence_repository
            .get_by_id(recurrence_id)
            .await?
            .ok_or_else(|| {
                Box::new(http_error::HttpError::not_found("recurrence", recurrence_id))
            })?;

        if recurrence.client_id() != &client_id {
            return Err(Box::new(http_error::HttpError::not_found(
                "recurrence",
                recurrence_id,
            )));
        }

        recurrence.update(
            request.description,
            request.day_of_month,
            request.end_date,
            request.active,
        );

        self.recurrence_repository.update(recurrence).await
    }

    async fn create_debt_recurrence(
        &self,
        client_id: Uuid,
        request: CreateRecurrenceRequest,
    ) -> HttpResult<Recurrence> {
        let recurrence = Recurrence::from_request(client_id, request);
        self.recurrence_repository.insert(recurrence).await
    }

    async fn update_debt(
        &self,
        client_id: Uuid,
        debt_id: Uuid,
        request: UpdateDebtRequest,
    ) -> HttpResult<Debt> {
        let mut debt = self
            .debt_repository
            .get_by_id(&debt_id)
            .await?
            .or_not_found("debt", debt_id.to_string())?;

        if debt.client_id() != &client_id {
            return Err(Box::new(http_error::HttpError::forbidden(
                "You don't have permission to update this debt",
            )));
        }

        if let Some(category) = request.category {
            debt.set_category(category);
        }
        if let Some(expense_type) = request.expense_type {
            debt.set_expense_type(expense_type);
        }
        if let Some(tags) = request.tags {
            debt.set_tags(tags);
        }
        if let Some(description) = request.description {
            debt.set_description(description);
        }
        if let Some(due_date) = request.due_date {
            debt.set_due_date(due_date);
        }

        self.debt_repository.update(debt).await
    }

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
        let debt = self.create_debt(client_id, request.clone()).await?;

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
    use http_error::{HttpError, HttpResult};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::debt::{DebtCategory, DebtStatus, ExpenseType};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateDebtRequest {
        pub category: Option<DebtCategory>,
        pub expense_type: Option<ExpenseType>,
        pub tags: Option<Vec<String>>,
        pub description: String,
        pub due_date: NaiveDate,
        pub total_amount: Decimal,
        pub paid_amount: Option<Decimal>,
        pub discount_amount: Option<Decimal>,
        pub status: Option<DebtStatus>,
        pub financial_instrument_id: Option<Uuid>,
        pub installment_count: Option<i32>,
    }

    impl CreateDebtRequest {
        pub fn new(
            category: Option<DebtCategory>,
            tags: Option<Vec<String>>,
            description: String,
            total_amount: Decimal,
            due_date: NaiveDate,
            installment_count: Option<i32>,
        ) -> Self {
            Self {
                category,
                expense_type: None,
                tags,
                description,
                total_amount,
                paid_amount: None,
                discount_amount: None,
                due_date,
                status: Some(DebtStatus::Open),
                financial_instrument_id: None,
                installment_count,
            }
        }

        pub fn validate(&self) -> HttpResult<()> {
            if self.invalid_total_amount() {
                return Err(Box::new(HttpError::bad_request(
                    "Total amount must be greater than zero",
                )));
            }

            if self.invalid_installment() {
                return Err(Box::new(HttpError::bad_request(
                    "Installment count and financial instrument must be provided for installment debts",
                )));
            }

            Ok(())
        }

        fn invalid_installment(&self) -> bool {
            self.installment_count.is_some() && self.financial_instrument_id.is_none()
        }

        fn invalid_total_amount(&self) -> bool {
            self.total_amount <= Decimal::ZERO
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateCategoryRequest {
        pub name: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateDebtRequest {
        pub category: Option<DebtCategory>,
        pub expense_type: Option<ExpenseType>,
        pub tags: Option<Vec<String>>,
        pub description: Option<String>,
        pub due_date: Option<NaiveDate>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateRecurrenceRequest {
        pub description: String,
        pub amount: Decimal,
        pub start_date: NaiveDate,
        pub end_date: Option<NaiveDate>,
        pub day_of_month: i32,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateRecurrenceRequest {
        pub description: Option<String>,
        pub day_of_month: Option<i32>,
        pub end_date: Option<NaiveDate>,
        pub active: Option<bool>,
    }
}
