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
    /// Unique identifier
    id: Uuid,
    /// Unique identifier of the debt
    /// The debt that the payment belongs to
    debt_id: Uuid,
    /// Unique identifier of the account
    /// The account that the payment belongs to
    /// a.k.a. "Conta de pagamento"
    account_id: Uuid,

    /// The amount of the payment
    amount: Decimal,
    /// The date of the payment
    payment_date: NaiveDate,

    /// The date of the creation of the payment
    created_at: DateTime<Utc>,
    /// The date of the last update of the payment
    updated_at: Option<DateTime<Utc>>,
}

impl Payment {
    pub fn new(debt: &Debt, account_id: &Uuid, payment_data: &PaymentBasicData) -> Self {
        Self {
            id: Uuid::new_v4(),
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
        debt_id: Uuid,
        account_id: Uuid,
        amount: Decimal,
        payment_date: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
