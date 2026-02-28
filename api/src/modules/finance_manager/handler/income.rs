use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::income::Income,
    handler::income::use_cases::CreateIncomeRequest,
    repository::income::{use_cases::IncomeListFilters, DynIncomeRepository},
};

#[async_trait]
pub trait IncomeHandler {
    async fn list_incomes(
        &self,
        client_id: Uuid,
        filters: IncomeListFilters,
    ) -> HttpResult<Vec<Income>>;
    async fn create_income(
        &self,
        client_id: Uuid,
        request: CreateIncomeRequest,
    ) -> HttpResult<Income>;
}

pub type DynIncomeHandler = dyn IncomeHandler + Send + Sync;

#[derive(Clone)]
pub struct IncomeHandlerImpl {
    pub income_repository: Arc<DynIncomeRepository>,
}

#[async_trait]
impl IncomeHandler for IncomeHandlerImpl {
    async fn list_incomes(
        &self,
        client_id: Uuid,
        filters: IncomeListFilters,
    ) -> HttpResult<Vec<Income>> {
        let filters = filters.with_client_id(client_id);
        self.income_repository.list(&filters).await
    }

    async fn create_income(
        &self,
        client_id: Uuid,
        request: CreateIncomeRequest,
    ) -> HttpResult<Income> {
        let income = Income::from_request(request, client_id);
        let income = self.income_repository.insert(income).await?;

        Ok(income)
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateIncomeRequest {
        pub financial_instrument_id: Uuid,
        pub description: String,
        pub amount: Decimal,
        pub date_reference: NaiveDate,
    }
}
