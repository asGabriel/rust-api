use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

use crate::modules::finance_manager::domain::payment::Payment;

pub mod generator;

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
    due_date: NaiveDate,

    /// The status of the debt
    #[serde(default)]
    status: DebtStatus,

    /// The date of the creation of the debt
    created_at: DateTime<Utc>,
    /// The date of the last update of the debt
    updated_at: Option<DateTime<Utc>>,
}

impl Debt {
    pub fn new(
        account_id: Uuid,
        description: String,
        total_amount: Decimal,
        paid_amount: Option<Decimal>,
        discount_amount: Option<Decimal>,
        due_date: NaiveDate,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            account_id,
            description,
            total_amount,
            paid_amount: paid_amount.unwrap_or(Decimal::ZERO),
            discount_amount: discount_amount.unwrap_or(Decimal::ZERO),
            remaining_amount: total_amount
                - paid_amount.unwrap_or(Decimal::ZERO)
                - discount_amount.unwrap_or(Decimal::ZERO),
            due_date,
            status: DebtStatus::default(),
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn paid(&mut self, payment: Payment) {
        self.paid_amount += payment.principal_amount();

        self.recalculate_remaining_amount();
        self.updated_at = Some(Utc::now());
    }

    fn recalculate_remaining_amount(&mut self) {
        self.remaining_amount = self.total_amount - self.paid_amount - self.discount_amount;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DebtStatus {
    /// The debt is unpaid; a.k.a. "Nova dívida"
    #[default]
    Unpaid,
    /// The debt is partially paid; a.k.a. "Dívida parcialmente paga"
    PartiallyPaid,
    /// The debt is settled; a.k.a. "Dívida paga"
    Settled,
}

impl From<String> for DebtStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "UNPAID" => DebtStatus::Unpaid,
            "PARTIALLY_PAID" => DebtStatus::PartiallyPaid,
            "SETTLED" => DebtStatus::Settled,
            _ => DebtStatus::default(),
        }
    }
}

impl From<&str> for DebtStatus {
    fn from(s: &str) -> Self {
        match s {
            "UNPAID" => DebtStatus::Unpaid,
            "PARTIALLY_PAID" => DebtStatus::PartiallyPaid,
            "SETTLED" => DebtStatus::Settled,
            _ => DebtStatus::default(),
        }
    }
}

impl From<DebtStatus> for String {
    fn from(status: DebtStatus) -> Self {
        match status {
            DebtStatus::Unpaid => "UNPAID".to_string(),
            DebtStatus::PartiallyPaid => "PARTIALLY_PAID".to_string(),
            DebtStatus::Settled => "SETTLED".to_string(),
        }
    }
}

getters!(
    Debt {
        id: Uuid,
        account_id: Uuid,
        description: String,
        total_amount: Decimal,
        paid_amount: Decimal,
        discount_amount: Decimal,
        remaining_amount: Decimal,
        due_date: NaiveDate,
        status: DebtStatus,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
);

from_row_constructor! {
    Debt {
        id: Uuid,
        account_id: Uuid,
        description: String,
        total_amount: Decimal,
        paid_amount: Decimal,
        discount_amount: Decimal,
        remaining_amount: Decimal,
        due_date: NaiveDate,
        status: DebtStatus,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DebtFilters {
    ids: Option<Vec<Uuid>>,
    statuses: Option<Vec<DebtStatus>>,
}
