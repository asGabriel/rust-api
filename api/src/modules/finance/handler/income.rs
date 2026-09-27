use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use uuid::Uuid;

use util::DeletedBy;

use crate::modules::finance::{
    domain::income::{Income, IncomeFilters},
    handler::income::use_cases::{CreateIncomeRequest, UpdateIncomeRequest},
    repository::income::DynIncomeRepository,
};
use std::sync::Arc;

pub type DynIncomeHandler = dyn IncomeHandler + Send + Sync;

#[async_trait]
pub trait IncomeHandler {
    async fn register_new_income(
        &self,
        client_id: Uuid,
        request: CreateIncomeRequest,
    ) -> HttpResult<Income>;

    async fn list_incomes(
        &self,
        client_id: Uuid,
        filters: &IncomeFilters,
    ) -> HttpResult<Vec<Income>>;

    async fn update_income(
        &self,
        client_id: Uuid,
        income_id: Uuid,
        request: UpdateIncomeRequest,
    ) -> HttpResult<Income>;

    async fn soft_delete_income(
        &self,
        client_id: Uuid,
        user_id: Uuid,
        income_id: Uuid,
    ) -> HttpResult<()>;
}

#[derive(Clone)]
pub struct IncomeHandlerImpl {
    pub income_repository: Arc<DynIncomeRepository>,
}

#[async_trait]
impl IncomeHandler for IncomeHandlerImpl {
    async fn register_new_income(
        &self,
        client_id: Uuid,
        request: CreateIncomeRequest,
    ) -> HttpResult<Income> {
        let income = Income::new(
            client_id,
            request.description,
            request.amount,
            request.received_date,
            request.category,
        )?;

        self.income_repository.insert(income).await
    }

    async fn list_incomes(
        &self,
        client_id: Uuid,
        filters: &IncomeFilters,
    ) -> HttpResult<Vec<Income>> {
        let built = IncomeFilters::new(client_id)
            .with_optional_categories(filters.categories().clone())
            .with_optional_start_date(*filters.start_date())
            .with_optional_end_date(*filters.end_date());

        self.income_repository.list(&built).await
    }

    async fn update_income(
        &self,
        client_id: Uuid,
        income_id: Uuid,
        request: UpdateIncomeRequest,
    ) -> HttpResult<Income> {
        let mut income = self
            .income_repository
            .get_by_id(&income_id)
            .await?
            .or_not_found("income", income_id.to_string())?;

        if income.client_id() != &client_id {
            return Err(Box::new(HttpError::forbidden(
                "You don't have permission to update this income",
            )));
        }

        if let Some(category) = request.category {
            income.set_category(category);
        }
        if let Some(description) = request.description {
            income.set_description(description);
        }
        if let Some(amount) = request.amount {
            income.set_amount(amount)?;
        }
        if let Some(received_date) = request.received_date {
            income.set_received_date(received_date);
        }

        self.income_repository.update(income).await
    }

    async fn soft_delete_income(
        &self,
        client_id: Uuid,
        user_id: Uuid,
        income_id: Uuid,
    ) -> HttpResult<()> {
        let income = self
            .income_repository
            .get_by_id(&income_id)
            .await?
            .or_not_found("income", income_id.to_string())?;

        if income.client_id() != &client_id {
            return Err(Box::new(HttpError::forbidden(
                "You don't have permission to delete this income",
            )));
        }

        self.income_repository
            .soft_delete(income_id, DeletedBy::new(user_id))
            .await
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    use crate::modules::finance::domain::income::IncomeCategory;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateIncomeRequest {
        pub category: Option<IncomeCategory>,
        pub description: String,
        pub amount: Decimal,
        pub received_date: NaiveDate,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateIncomeRequest {
        pub category: Option<IncomeCategory>,
        pub description: Option<String>,
        pub amount: Option<Decimal>,
        pub received_date: Option<NaiveDate>,
    }
}
