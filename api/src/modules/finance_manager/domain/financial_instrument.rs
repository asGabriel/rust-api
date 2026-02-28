use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

pub mod configuration;

use crate::modules::finance_manager::{
    domain::financial_instrument::configuration::InstrumentConfiguration,
    handler::financial_instrument::use_cases::UpdateFinancialInstrumentRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FinancialInstrumentType {
    CreditCard,
    DebitAccount,
    InvestmentBox,
}

impl FinancialInstrumentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FinancialInstrumentType::CreditCard => "CREDIT_CARD",
            FinancialInstrumentType::DebitAccount => "DEBIT_ACCOUNT",
            FinancialInstrumentType::InvestmentBox => "INVESTMENT_BOX",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "CREDIT_CARD" => FinancialInstrumentType::CreditCard,
            "DEBIT_ACCOUNT" => FinancialInstrumentType::DebitAccount,
            "INVESTMENT_BOX" => FinancialInstrumentType::InvestmentBox,
            _ => FinancialInstrumentType::DebitAccount,
        }
    }

    pub fn requires_due_date_configuration(&self) -> bool {
        matches!(self, FinancialInstrumentType::CreditCard)
    }
}

impl Default for FinancialInstrumentType {
    fn default() -> Self {
        Self::DebitAccount
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialInstrument {
    id: Uuid,
    client_id: Uuid,
    name: String,
    owner: String,
    identification: String,
    instrument_type: FinancialInstrumentType,
    configuration: InstrumentConfiguration,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl FinancialInstrument {
    pub fn new(
        client_id: Uuid,
        name: String,
        owner: String,
        instrument_type: FinancialInstrumentType,
        configuration: InstrumentConfiguration,
    ) -> Self {
        let uuid = Uuid::new_v4();

        Self {
            id: uuid,
            client_id,
            name,
            owner,
            identification: String::new(),
            instrument_type,
            configuration,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn default_due_date(&self) -> Option<NaiveDate> {
        self.configuration.default_due_date()
    }

    pub fn update(&mut self, request: &UpdateFinancialInstrumentRequest) {
        if let Some(name) = &request.name {
            self.name = name.clone();
        }
        if let Some(owner) = &request.owner {
            self.owner = owner.clone();
        }
        if let Some(instrument_type) = &request.instrument_type {
            self.instrument_type = instrument_type.clone();
        }
        if let Some(configuration) = &request.configuration {
            self.configuration = configuration.clone();
        }

        self.updated_at = Some(Utc::now());
    }
}

getters! {
    FinancialInstrument {
        id: Uuid,
        client_id: Uuid,
        name: String,
        owner: String,
        identification: String,
        instrument_type: FinancialInstrumentType,
        configuration: InstrumentConfiguration,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

from_row_constructor! {
    FinancialInstrument {
        id: Uuid,
        client_id: Uuid,
        name: String,
        owner: String,
        identification: String,
        instrument_type: FinancialInstrumentType,
        configuration: InstrumentConfiguration,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
