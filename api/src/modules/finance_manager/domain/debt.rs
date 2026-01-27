use chrono::{DateTime, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use util::{from_row_constructor, getters};
use uuid::Uuid;

use crate::modules::{
    chat_bot::domain::formatter::{ChatFormatter, ChatFormatterUtils},
    finance_manager::domain::{debt::installment::Installment, payment::Payment},
};

pub mod category;
pub mod installment;
pub mod recurrence;
pub mod recurrence_run;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Debt {
    id: Uuid,
    client_id: Uuid,
    category: DebtCategory,
    expense_type: ExpenseType,
    tags: Vec<String>,
    identification: String,
    description: String,
    total_amount: Decimal,
    paid_amount: Decimal,
    discount_amount: Decimal,
    remaining_amount: Decimal,
    due_date: NaiveDate,
    #[serde(default)]
    status: DebtStatus,
    #[serde(default)]
    payment_progress: PaymentProgress,
    installment_count: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Debt {
    pub fn new(
        client_id: Uuid,
        description: String,
        total_amount: Decimal,
        paid_amount: Option<Decimal>,
        discount_amount: Option<Decimal>,
        due_date: NaiveDate,
        category: Option<DebtCategory>,
        expense_type: Option<ExpenseType>,
        tags: Option<Vec<String>>,
        installment_count: Option<i32>,
    ) -> Self {
        let uuid = Uuid::new_v4();
        let remaining_amount = total_amount
            - paid_amount.unwrap_or(Decimal::ZERO)
            - discount_amount.unwrap_or(Decimal::ZERO);

        Self {
            id: uuid,
            client_id,
            category: category.unwrap_or_default(),
            expense_type: expense_type.unwrap_or_default(),
            tags: tags.unwrap_or_default(),
            identification: String::new(),
            description,
            total_amount,
            paid_amount: paid_amount.unwrap_or(Decimal::ZERO),
            discount_amount: discount_amount.unwrap_or(Decimal::ZERO),
            remaining_amount,
            due_date,
            status: DebtStatus::default(),
            installment_count,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn process_payment(&mut self, payment: &Payment) -> HttpResult<()> {
        self.validate_payment_amount(payment)?;

        self.paid_amount += payment.amount();

        self.recalculate_remaining_amount();
        self.recalculate_status();
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    pub fn reconcile_with_actual_payment(&mut self, payment: &Payment) -> HttpResult<()> {
        self.total_amount = *payment.amount();
        self.paid_amount = *payment.amount();
        self.remaining_amount = Decimal::ZERO;
        self.recalculate_status();

        self.updated_at = Some(Utc::now());

        Ok(())
    }

    /// Generates the installments of the debt
    /// It will return the installments if the debt is parceled, otherwise it will return None
    pub fn generate_installments(&self) -> HttpResult<Option<Vec<Installment>>> {
        let installment_count = match self.installment_count {
            Some(count) if count > 0 => count,
            _ => return Ok(None),
        };

        let (base_amount, remainder) = self.calculate_installment_amount(installment_count);

        let mut installments = Vec::new();

        for i in 1..=installment_count {
            // the first installment receives the remainder to avoid rounding errors
            let amount = if i == 1 {
                base_amount + remainder
            } else {
                base_amount
            };

            let due_date = self
                .due_date
                .checked_add_months(chrono::Months::new((i - 1) as u32))
                .ok_or_else(|| {
                    Box::new(HttpError::bad_request(format!(
                        "Could not calculate due date for installment {}",
                        i
                    )))
                })?;

            installments.push(Installment::new(*self.id(), i, due_date, amount));
        }

        Ok(Some(installments))
    }

    pub fn has_installments(&self) -> bool {
        self.installment_count.is_some() && self.installment_count.unwrap() > 0
    }

    /// Returns the installment amount if the debt has installments,
    /// otherwise returns the remaining amount
    pub fn installment_amount(&self) -> Decimal {
        match self.installment_count {
            Some(count) if count > 1 => {
                let (base_amount, _) = self.calculate_installment_amount(count);
                base_amount
            }
            _ => self.remaining_amount,
        }
    }

    // PRIVATE METHODS

    /// Checks if the payment amount is valid to be processed
    pub fn validate_payment_amount(&self, payment: &Payment) -> HttpResult<()> {
        if self.paid_amount >= self.total_amount {
            return Err(Box::new(HttpError::bad_request("Debt already paid")));
        }

        if *payment.amount() > self.remaining_amount {
            return Err(Box::new(HttpError::bad_request(format!(
                "Payment amount ({:.2}) exceeds remaining amount ({:.2})",
                payment.amount(),
                self.remaining_amount
            ))));
        }

        Ok(())
    }

    fn recalculate_remaining_amount(&mut self) {
        self.remaining_amount = self.total_amount - self.paid_amount - self.discount_amount;
    }

    fn recalculate_status(&mut self) {
        if self.paid_amount >= self.total_amount {
            self.status = DebtStatus::Settled;
        } else if self.remaining_amount > Decimal::ZERO {
            self.status = DebtStatus::PartiallyPaid;
        } else {
            self.status = DebtStatus::Unpaid;
        }
    }

    /// Calculates the amount of the installment and the remainder
    fn calculate_installment_amount(&self, installment_number: i32) -> (Decimal, Decimal) {
        let installment_number = Decimal::from(installment_number);

        let base_amount = self.total_amount() / installment_number;
        let remainder = self.total_amount() - (base_amount * installment_number);
        (base_amount, remainder)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DebtCategory {
    #[default]
    Unknown,
    Home,
    Transport,
    Health,
    Food,
    Lifestyle,
    Education,
    Goals,
    Personal,
}

impl From<String> for DebtCategory {
    fn from(s: String) -> Self {
        match s.as_str() {
            "HOME" => DebtCategory::Home,
            "TRANSPORT" => DebtCategory::Transport,
            "HEALTH" => DebtCategory::Health,
            "FOOD" => DebtCategory::Food,
            "LIFESTYLE" => DebtCategory::Lifestyle,
            "EDUCATION" => DebtCategory::Education,
            "GOALS" => DebtCategory::Goals,
            "PERSONAL" => DebtCategory::Personal,
            _ => DebtCategory::Unknown,
        }
    }
}

impl From<DebtCategory> for String {
    fn from(category: DebtCategory) -> Self {
        match category {
            DebtCategory::Home => "HOME".to_string(),
            DebtCategory::Transport => "TRANSPORT".to_string(),
            DebtCategory::Health => "HEALTH".to_string(),
            DebtCategory::Food => "FOOD".to_string(),
            DebtCategory::Lifestyle => "LIFESTYLE".to_string(),
            DebtCategory::Education => "EDUCATION".to_string(),
            DebtCategory::Goals => "GOALS".to_string(),
            DebtCategory::Personal => "PERSONAL".to_string(),
            DebtCategory::Unknown => "UNKNOWN".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpenseType {
    Fixed,
    #[default]
    Variable,
}

impl ExpenseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExpenseType::Fixed => "FIXED",
            ExpenseType::Variable => "VARIABLE",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "FIXED" => ExpenseType::Fixed,
            "VARIABLE" => ExpenseType::Variable,
            _ => ExpenseType::Variable,
        }
    }
}

/// Represents the temporal status of a debt
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DebtStatus {
    /// No overdue payments. A.k.a. "Em dia"
    #[default]
    Current,
    /// Has overdue amount. A.k.a. "Vencida"
    Overdue,
    /// Fully paid. A.k.a. "Quitada"
    Settled
}

/// Represents the progress of a payment
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PaymentProgress {
    /// Nothing paid yet
    #[default]
    Unpaid,
    /// Partially paid (with remaining balance)
    PartiallyPaid,
    /// Fully paid
    Paid,
}



impl From<String> for DebtStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "CURRENT" => DebtStatus::Current,
            "OVERDUE" => DebtStatus::Overdue,
            "SETTLED" => DebtStatus::Settled,
            _ => DebtStatus::default(),
        }
    }
}

impl From<&str> for DebtStatus {
    fn from(s: &str) -> Self {
        let s_upper = s.to_uppercase();
        match s_upper.as_str() {
            // Valores em inglês (banco de dados)
            "CURRENT" => DebtStatus::Current,
            "OVERDUE" => DebtStatus::Overdue,
            "SETTLED" => DebtStatus::Settled,
            // Valores em português (interface do usuário)
            "PENDENTE" => DebtStatus::Current,
            "VENCIDA" => DebtStatus::Overdue,
            "PAGO" => DebtStatus::Settled,
            _ => DebtStatus::default(),
        }
    }
}

impl From<DebtStatus> for String {
    fn from(status: DebtStatus) -> Self {
        match status {
            DebtStatus::Current => "CURRENT".to_string(),
            DebtStatus::Overdue => "OVERDUE".to_string(),
            DebtStatus::Settled => "SETTLED".to_string(),
        }
    }
}

impl std::fmt::Display for DebtStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DebtStatus::Current => "CURRENT",
            DebtStatus::Overdue => "OVERDUE",
            DebtStatus::Settled => "SETTLED",
        };
        write!(f, "{}", s)
    }
}

impl DebtStatus {
    pub fn emoji(&self) -> &'static str {
        match self {
            DebtStatus::Current => "🟢",
            DebtStatus::Overdue => "🔴",
            DebtStatus::Settled => "🟢",
        }
    }

    pub fn to_pt_br(&self) -> &'static str {
        match self {
            DebtStatus::Current => "Em dia",
            DebtStatus::Overdue => "Vencida",
            DebtStatus::Settled => "Quitada",
            DebtStatus::Settled => "Pago",
        }
    }
}

getters!(
    Debt {
        id: Uuid,
        client_id: Uuid,
        category: DebtCategory,
        expense_type: ExpenseType,
        tags: Vec<String>,
        identification: String,
        description: String,
        total_amount: Decimal,
        paid_amount: Decimal,
        discount_amount: Decimal,
        remaining_amount: Decimal,
        due_date: NaiveDate,
        status: DebtStatus,
        installment_count: Option<i32>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
);

impl Debt {
    pub fn set_category(&mut self, category: DebtCategory) {
        self.category = category;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_expense_type(&mut self, expense_type: ExpenseType) {
        self.expense_type = expense_type;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_tags(&mut self, tags: Vec<String>) {
        self.tags = tags;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_description(&mut self, description: String) {
        self.description = description;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_due_date(&mut self, due_date: NaiveDate) {
        self.due_date = due_date;
        self.updated_at = Some(Utc::now());
    }
}

from_row_constructor! {
    Debt {
        id: Uuid,
        client_id: Uuid,
        category: DebtCategory,
        expense_type: ExpenseType,
        tags: Vec<String>,
        identification: String,
        description: String,
        total_amount: Decimal,
        paid_amount: Decimal,
        discount_amount: Decimal,
        remaining_amount: Decimal,
        due_date: NaiveDate,
        status: DebtStatus,
        installment_count: Option<i32>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DebtFilters {
    client_id: Option<Uuid>,
    ids: Option<Vec<Uuid>>,
    statuses: Option<Vec<DebtStatus>>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
    category_names: Option<Vec<String>>,
}

getters!(
    DebtFilters {
        client_id: Option<Uuid>,
        ids: Option<Vec<Uuid>>,
        statuses: Option<Vec<DebtStatus>>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        category_names: Option<Vec<String>>,
    }
);

impl DebtFilters {
    pub fn new(client_id: Uuid) -> Self {
        Self {
            client_id: Some(client_id),
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

    pub fn with_category_names(mut self, category_names: Vec<String>) -> Self {
        self.category_names = Some(
            category_names
                .into_iter()
                .map(|name| name.to_uppercase())
                .collect(),
        );
        self
    }
}

impl ChatFormatter for Debt {
    /// Formats a single debt for chat display
    fn format_for_chat(&self) -> String {
        let mut output = String::new();

        writeln!(output, "💰 Débitos de {}", self.description()).unwrap();
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
            return "📝 Nenhuma despesa encontrada".to_string();
        }

        let mut output = String::new();
        // Summary
        let total_remaining: Decimal = items.iter().map(|d| *d.remaining_amount()).sum();
        let total_paid: Decimal = items.iter().map(|d| *d.paid_amount()).sum();

        writeln!(
            output,
            "\n✅{} Total pago\n🔴{} Total em aberto\n\n ######\n",
            ChatFormatterUtils::format_currency(&total_paid),
            ChatFormatterUtils::format_currency(&total_remaining)
        )
        .unwrap();

        let mut sorted_items: Vec<&Debt> = items.iter().collect();
        sorted_items.sort_by(|a, b| {
            let a_is_paid = a.status() == &DebtStatus::Settled;
            let b_is_paid = b.status() == &DebtStatus::Settled;

            match (a_is_paid, b_is_paid) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.due_date().cmp(b.due_date()),
            }
        });

        for debt in sorted_items.iter() {
            let (value, due_date) = if *debt.remaining_amount() > Decimal::ZERO {
                (debt.remaining_amount(), *debt.due_date())
            } else {
                (
                    debt.paid_amount(),
                    debt.updated_at().unwrap_or(Utc::now()).naive_utc().date(),
                )
            };

            let date_str = due_date.format("%d/%m").to_string();

            // Formata valor considerando parcelas (valor da parcela é o principal)
            let value_display = match debt.installment_count() {
                Some(count) if *count > 1 => {
                    let installment_value = value / Decimal::from(*count);
                    format!(
                        "💵{:.0} ({count}x total 💵{:.0})",
                        installment_value,
                        value,
                        count = count
                    )
                }
                _ => format!("💵{:.0}", value),
            };

            writeln!(
                output,
                "{}{}: {} {} - {}",
                debt.status().emoji(),
                debt.identification(),
                date_str,
                value_display,
                debt.description(),
            )
            .unwrap();
        }

        output
    }
}
