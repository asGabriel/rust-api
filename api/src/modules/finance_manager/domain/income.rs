use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

use crate::modules::{
    chat_bot::domain::formatter::ChatFormatter,
    finance_manager::handler::income::use_cases::CreateIncomeRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Income {
    id: Uuid,
    account_id: Uuid,
    description: String,
    amount: Decimal,
    reference: NaiveDate,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Income {
    pub fn from_request(request: CreateIncomeRequest, account_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            account_id: account_id,
            description: request.description,
            amount: request.amount,
            reference: request.date_reference,
            created_at: Utc::now(),
            updated_at: None,
        }
    }
}

getters! {
    Income {
        id: Uuid,
        account_id: Uuid,
        description: String,
        amount: Decimal,
        reference: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

from_row_constructor! {
    Income {
        id: Uuid,
        account_id: Uuid,
        description: String,
        amount: Decimal,
        reference: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

impl ChatFormatter for Income {
    fn format_for_chat(&self) -> String {
        format!(
            "{} - {} - {}",
            self.description(),
            self.amount(),
            self.reference().format("%d/%m/%Y"),
        )
    }

    fn format_list_for_chat(items: &[Self]) -> String {
        let mut output = format!("📋 Receitas cadastradas ({})", items.len());

        for income in items.iter() {
            output.push_str(&format!("\n{}", income.format_for_chat()));
        }

        output
    }
}
