use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};

use crate::modules::finance_manager::{
    domain::income::Income,
    handler::income::use_cases::CreateIncomeRequest,
    repository::{account::DynAccountRepository, income::DynIncomeRepository},
};

#[async_trait]
pub trait IncomeHandler {
    async fn list_incomes(&self) -> HttpResult<Vec<Income>>;
    async fn create_income(&self, request: CreateIncomeRequest) -> HttpResult<Income>;
}

pub type DynIncomeHandler = dyn IncomeHandler + Send + Sync;

#[derive(Clone)]
pub struct IncomeHandlerImpl {
    pub income_repository: Arc<DynIncomeRepository>,
    pub account_repository: Arc<DynAccountRepository>,
}

#[async_trait]
impl IncomeHandler for IncomeHandlerImpl {
    async fn list_incomes(&self) -> HttpResult<Vec<Income>> {
        self.income_repository.list().await
    }

    async fn create_income(&self, request: CreateIncomeRequest) -> HttpResult<Income> {
        let account = self
            .account_repository
            .get_by_identification(&request.account_identification)
            .await?
            .or_not_found("account", &request.account_identification)?;

        let income = Income::from_request(request, *account.id());
        let income = self.income_repository.insert(income).await?;

        Ok(income)
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateIncomeRequest {
        pub account_identification: String,
        pub description: String,
        pub amount: Decimal,
        pub date_reference: NaiveDate,
    }
}
