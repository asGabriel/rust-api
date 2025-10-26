use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};

use crate::modules::finance_manager::{
    domain::recurrence::Recurrence,
    handler::recurrence::use_cases::CreateRecurrenceRequest,
    repository::{account::DynAccountRepository, recurrence::DynRecurrenceRepository},
};

pub type DynRecurrenceHandler = dyn RecurrenceHandler + Send + Sync;

#[derive(Clone)]
pub struct RecurrenceHandlerImpl {
    pub recurrence_repository: Arc<DynRecurrenceRepository>,
    pub account_repository: Arc<DynAccountRepository>,
}

impl RecurrenceHandlerImpl {
    pub fn new(
        recurrence_repository: Arc<DynRecurrenceRepository>,
        account_repository: Arc<DynAccountRepository>,
    ) -> Self {
        Self {
            recurrence_repository,
            account_repository,
        }
    }
}

#[async_trait]
pub trait RecurrenceHandler {
    async fn create_recurrence(&self, request: CreateRecurrenceRequest) -> HttpResult<Recurrence>;
    async fn list_recurrences(&self) -> HttpResult<Vec<Recurrence>>;
}

#[async_trait]
impl RecurrenceHandler for RecurrenceHandlerImpl {
    async fn list_recurrences(&self) -> HttpResult<Vec<Recurrence>> {
        self.recurrence_repository.list().await
    }

    async fn create_recurrence(&self, request: CreateRecurrenceRequest) -> HttpResult<Recurrence> {
        let account = self
            .account_repository
            .get_by_identification(&request.account_identification)
            .await?
            .or_not_found("account", &request.account_identification)?;

        let recurrence = Recurrence::from_request(request, *account.id());
        let recurrence_created = self.recurrence_repository.insert(recurrence).await?;

        Ok(recurrence_created)
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateRecurrenceRequest {
        pub account_identification: String,
        pub description: String,
        pub amount: Decimal,
        pub start_date: NaiveDate,
        pub end_date: Option<NaiveDate>,
        pub day_of_month: i32,
    }
}
