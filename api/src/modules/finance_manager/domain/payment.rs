use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::debt::Debt, handler::payment::use_cases::PaymentBasicData,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    id: Uuid,
    client_id: Uuid,
    debt_id: Uuid,
    account_id: Uuid,
    amount: Decimal,
    payment_date: NaiveDate,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Payment {
    pub fn new(debt: &Debt, account_id: &Uuid, payment_data: &PaymentBasicData) -> Self {
        Self {
            id: Uuid::new_v4(),
            client_id: *debt.client_id(),
            debt_id: *debt.id(),
            account_id: *account_id,
            amount: payment_data.amount(debt),
            payment_date: payment_data.payment_date,
            created_at: Utc::now(),
            updated_at: None,
        }
    }
}

getters! {
    Payment {
        id: Uuid,
        client_id: Uuid,
        debt_id: Uuid,
        account_id: Uuid,
        amount: Decimal,
        payment_date: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

from_row_constructor! {
    Payment {
        id: Uuid,
        client_id: Uuid,
        debt_id: Uuid,
        account_id: Uuid,
        amount: Decimal,
        payment_date: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
