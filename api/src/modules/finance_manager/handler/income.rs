use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::income::Income,
    handler::income::use_cases::{CreateIncomeRequest, ListIncomesRequest},
    repository::income::{use_cases::IncomeListFilters, DynIncomeRepository},
};

#[async_trait]
pub trait IncomeHandler {
    async fn list_incomes(
        &self,
        client_id: Uuid,
        filters: ListIncomesRequest,
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
        filters: ListIncomesRequest,
    ) -> HttpResult<Vec<Income>> {
        let filters = IncomeListFilters::new(client_id)
            .with_financial_instrument_ids(filters.financial_instrument_ids)
            .with_start_date(filters.start_date)
            .with_end_date(filters.end_date);

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

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ListIncomesRequest {
        pub financial_instrument_ids: Option<Vec<Uuid>>,
        pub start_date: Option<NaiveDate>,
        pub end_date: Option<NaiveDate>,
    }
}
