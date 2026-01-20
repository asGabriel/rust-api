use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::financial_instrument::FinancialInstrument,
    handler::financial_instrument::use_cases::{
        CreateFinancialInstrumentRequest, FinancialInstrumentListFilters,
        UpdateFinancialInstrumentRequest,
    },
    repository::financial_instrument::DynFinancialInstrumentRepository,
};

pub type DynFinancialInstrumentHandler = dyn FinancialInstrumentHandler + Send + Sync;

#[async_trait]
pub trait FinancialInstrumentHandler {
    async fn create_financial_instrument(
        &self,
        client_id: Uuid,
        request: CreateFinancialInstrumentRequest,
    ) -> HttpResult<FinancialInstrument>;

    async fn list_financial_instruments(
        &self,
        client_id: Uuid,
        filters: FinancialInstrumentListFilters,
    ) -> HttpResult<Vec<FinancialInstrument>>;

    async fn update_financial_instrument(
        &self,
        client_id: Uuid,
        request: UpdateFinancialInstrumentRequest,
    ) -> HttpResult<FinancialInstrument>;
}

#[derive(Clone)]
pub struct FinancialInstrumentHandlerImpl {
    pub financial_instrument_repository: Arc<DynFinancialInstrumentRepository>,
}

#[async_trait]
impl FinancialInstrumentHandler for FinancialInstrumentHandlerImpl {
    async fn update_financial_instrument(
        &self,
        _client_id: Uuid,
        request: UpdateFinancialInstrumentRequest,
    ) -> HttpResult<FinancialInstrument> {
        let mut instrument = self
            .financial_instrument_repository
            .get_by_identification(&request.identification)
            .await?
            .or_not_found("financial_instrument", &request.identification)?;

        instrument.update(&request);
        self.financial_instrument_repository
            .update(instrument.clone())
            .await?;

        Ok(instrument)
    }

    async fn create_financial_instrument(
        &self,
        client_id: Uuid,
        request: CreateFinancialInstrumentRequest,
    ) -> HttpResult<FinancialInstrument> {
        let instrument_type = request.instrument_type.clone().unwrap_or_default();
        let configuration = request.configuration.clone().unwrap_or_default();

        if instrument_type.requires_due_date_configuration()
            && configuration.default_due_date.is_none()
        {
            return Err(Box::new(HttpError::bad_request(
                "Cartão de crédito requer configuração de data de vencimento",
            )));
        }

        let financial_instrument = FinancialInstrument::new(
            client_id,
            request.name,
            request.owner,
            instrument_type,
            configuration,
        );

        self.financial_instrument_repository
            .insert(financial_instrument)
            .await
    }

    async fn list_financial_instruments(
        &self,
        client_id: Uuid,
        filters: FinancialInstrumentListFilters,
    ) -> HttpResult<Vec<FinancialInstrument>> {
        let filters = filters.with_client_id(client_id);
        self.financial_instrument_repository.list(filters).await
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::financial_instrument::{
        configuration::InstrumentConfiguration, FinancialInstrumentType,
    };

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct FinancialInstrumentListFilters {
        pub client_id: Option<Uuid>,
        pub ids: Option<Vec<Uuid>>,
        pub identifications: Option<Vec<String>>,
        pub instrument_types: Option<Vec<FinancialInstrumentType>>,
    }

    impl FinancialInstrumentListFilters {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        pub fn with_client_id(mut self, client_id: Uuid) -> Self {
            self.client_id = Some(client_id);
            self
        }

        pub fn with_ids(mut self, ids: Vec<Uuid>) -> Self {
            self.ids = Some(ids);
            self
        }

        pub fn with_identifications(mut self, identifications: Vec<String>) -> Self {
            self.identifications = Some(identifications);
            self
        }

        pub fn with_instrument_types(
            mut self,
            instrument_types: Vec<FinancialInstrumentType>,
        ) -> Self {
            self.instrument_types = Some(instrument_types);
            self
        }
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateFinancialInstrumentRequest {
        pub name: String,
        pub owner: String,
        pub instrument_type: Option<FinancialInstrumentType>,
        pub configuration: Option<InstrumentConfiguration>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateFinancialInstrumentRequest {
        pub identification: String,
        pub name: Option<String>,
        pub owner: Option<String>,
        pub instrument_type: Option<FinancialInstrumentType>,
        pub configuration: Option<InstrumentConfiguration>,
    }
}
