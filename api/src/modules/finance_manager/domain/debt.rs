use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use util::{from_row_constructor, getters};
use uuid::Uuid;

use crate::{
    modules::{chat_bot::formatter::ChatFormatterUtils, finance_manager::domain::payment::Payment},
    utils::generate_random_identification,
};

pub mod generator;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Debt {
    /// Unique identifier
    id: Uuid,
    /// Unique identifier of the account
    /// The account that the debt belongs to
    account_id: Uuid,
    /// The identification of the debt for human readability
    identification: String,

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
        let uuid = Uuid::new_v4();
        let identification = generate_random_identification(uuid);

        Self {
            id: uuid,
            account_id,
            identification,
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
        identification: String,
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
        identification: String,
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
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

getters!(
    DebtFilters {
        ids: Option<Vec<Uuid>>,
        statuses: Option<Vec<DebtStatus>>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    }
);

impl DebtFilters {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn with_statuses(mut self, statuses: Vec<DebtStatus>) -> Self {
        self.statuses = Some(statuses);
        self
    }

    pub fn with_ids(mut self, ids: Vec<Uuid>) -> Self {
        self.ids = Some(ids);
        self
    }

    pub fn with_start_date(mut self, start_date: NaiveDate) -> Self {
        self.start_date = Some(start_date);
        self
    }

    pub fn with_end_date(mut self, end_date: NaiveDate) -> Self {
        self.end_date = Some(end_date);
        self
    }
}

impl crate::modules::chat_bot::formatter::ChatFormatter for Debt {
    /// Formats a single debt for chat display
    fn format_for_chat(&self) -> String {
        let mut output = String::new();

        writeln!(output, "💰 *Debt: {}*", self.description()).unwrap();
        writeln!(output, "🆔 ID: {}", self.identification()).unwrap();
        writeln!(
            output,
            "📅 Due Date: {}",
            ChatFormatterUtils::format_date(self.due_date())
        )
        .unwrap();
        writeln!(
            output,
            "💵 Total Amount: {}",
            ChatFormatterUtils::format_currency(self.total_amount())
        )
        .unwrap();
        writeln!(
            output,
            "✅ Paid Amount: {}",
            ChatFormatterUtils::format_currency(self.paid_amount())
        )
        .unwrap();
        writeln!(
            output,
            "🎯 Remaining Amount: {}",
            ChatFormatterUtils::format_currency(self.remaining_amount())
        )
        .unwrap();
        writeln!(
            output,
            "📊 Status: {}",
            ChatFormatterUtils::format_debt_status(self.status())
        )
        .unwrap();

        if let Some(updated_at) = self.updated_at() {
            writeln!(
                output,
                "🔄 Last Updated: {}",
                ChatFormatterUtils::format_datetime(updated_at)
            )
            .unwrap();
        }

        output
    }

    /// Formats debt list for chat display
    fn format_list_for_chat(items: &[Self]) -> String {
        if items.is_empty() {
            return "📝 *No debts found*".to_string();
        }

        let mut output = String::new();
        writeln!(output, "📋 *Debt List ({})*", items.len()).unwrap();

        for debt in items.iter() {
            writeln!(
                output,
                "\n🆔 *{}* - {}",
                debt.identification(),
                debt.description()
            )
            .unwrap();
            writeln!(
                output,
                "   💵 {} | 📅 {} | {}",
                ChatFormatterUtils::format_currency(debt.remaining_amount()),
                ChatFormatterUtils::format_date(debt.due_date()),
                ChatFormatterUtils::format_debt_status(debt.status())
            )
            .unwrap();
        }

        // Summary
        let total_remaining: Decimal = items.iter().map(|d| *d.remaining_amount()).sum();

        writeln!(
            output,
            "\n💼 *Total Outstanding: {}*",
            ChatFormatterUtils::format_currency(&total_remaining)
        )
        .unwrap();

        output
    }
}
