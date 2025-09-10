use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    pub id: Uuid,
    pub debt_id: Uuid,
    pub account_id: Uuid,

    /// The total amount of the payment
    pub total_amount: Decimal,
    /// The discount amount of the payment
    pub discount_amount: Decimal,
    /// The principal amount of the payment (amortized amount)
    pub principal_amount: Decimal,
    /// The date of the payment
    pub date: DateTime<Utc>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}
