use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};

use crate::modules::finance_manager::{
    domain::debt::recurrence::{Recurrence, RecurrenceFilters},
    handler::recurrence::use_cases::CreateRecurrenceRequest,
    repository::{financial_instrument::DynFinancialInstrumentRepository, recurrence::DynRecurrenceRepository},
};

pub type DynRecurrenceHandler = dyn RecurrenceHandler + Send + Sync;

#[derive(Clone)]
pub struct RecurrenceHandlerImpl {
    pub recurrence_repository: Arc<DynRecurrenceRepository>,
    pub financial_instrument_repository: Arc<DynFinancialInstrumentRepository>,
}

impl RecurrenceHandlerImpl {
    pub fn new(
        recurrence_repository: Arc<DynRecurrenceRepository>,
        financial_instrument_repository: Arc<DynFinancialInstrumentRepository>,
    ) -> Self {
        Self {
            recurrence_repository,
            financial_instrument_repository,
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
        self.recurrence_repository
            .list(&RecurrenceFilters::new())
            .await
    }

    async fn create_recurrence(&self, request: CreateRecurrenceRequest) -> HttpResult<Recurrence> {
        let instrument = self
            .financial_instrument_repository
            .get_by_identification(&request.financial_instrument_identification)
            .await?
            .or_not_found("financial_instrument", &request.financial_instrument_identification)?;

        let recurrence = Recurrence::from_request(request, *instrument.id());
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
        pub financial_instrument_identification: String,
        pub description: String,
        pub amount: Decimal,
        pub start_date: NaiveDate,
        pub end_date: Option<NaiveDate>,
        pub day_of_month: i32,
    }
}
