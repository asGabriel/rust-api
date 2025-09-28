use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
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
    settlement_account_id: Uuid,

    /// The total amount of the payment
    total_amount: Decimal,
    /// The principal amount of the payment (amortized amount)
    principal_amount: Decimal,
    /// The discount amount of the payment
    discount_amount: Decimal,
    /// The fine amount of the payment
    fine_amount: Decimal,
    /// The date of the payment
    date: DateTime<Utc>,

    /// The date of the creation of the payment
    created_at: DateTime<Utc>,
    /// The date of the last update of the payment
    updated_at: Option<DateTime<Utc>>,
}
