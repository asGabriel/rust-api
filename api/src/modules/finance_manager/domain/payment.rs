use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

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

    /// The total amount of the payment
    total_amount: Decimal,
    /// The principal amount of the payment (amortized amount)
    principal_amount: Decimal,
    /// The discount amount of the payment
    discount_amount: Decimal,
    /// The date of the payment
    payment_date: NaiveDate,

    /// The date of the creation of the payment
    created_at: DateTime<Utc>,
    /// The date of the last update of the payment
    updated_at: Option<DateTime<Utc>>,
}

impl Payment {
    pub fn new(
        debt_id: Uuid,
        account_id: Uuid,
        total_amount: Decimal,
        principal_amount: Decimal,
        discount_amount: Option<Decimal>,
        payment_date: NaiveDate,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            debt_id,
            account_id,
            total_amount,
            principal_amount,
            discount_amount: discount_amount.unwrap_or(Decimal::ZERO),
            payment_date,
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
        total_amount: Decimal,
        principal_amount: Decimal,
        discount_amount: Decimal,
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
        total_amount: Decimal,
        principal_amount: Decimal,
        discount_amount: Decimal,
        payment_date: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
