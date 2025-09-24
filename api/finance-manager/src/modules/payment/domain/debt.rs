use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Debt {
    /// Unique identifier
    id: Uuid,
    /// Unique identifier of the account
    /// The account that the debt belongs to
    account_id: Uuid,

    /// The description of the debt
    description: String,

    /// The total value of the debt
    total_amount: Decimal,
    /// The paid value of the debt
    paid_amount: Decimal,
    /// The discount amount of the debt
    discount_amount: Decimal,
    /// The remaining value of the debt
    remaining_amount: Decimal,
    /// The due date of the debt
    due_date: DateTime<Utc>,

    /// The status of the debt
    status: DebtStatus,

    /// The date of the creation of the debt
    created_at: DateTime<Utc>,
    /// The date of the last update of the debt
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DebtStatus {
    /// The debt is unpaid; a.k.a. "Nova dívida"
    #[default]
    Unpaid,
    /// The debt is partially paid; a.k.a. "Dívida parcialmente paga"
    PartiallyPaid,
    /// The debt is settled; a.k.a. "Dívida paga"
    Settled,
}