use chrono::{DateTime, Datelike, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{date::date_with_day_or_last, from_row_constructor, getters};
use uuid::Uuid;

use crate::modules::finance_manager::domain::{
    debt::installment::Installment, payment::Payment,
};

pub mod category;
pub mod installment;
pub mod recurrence;

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
    installment_count: Option<i32>,
    financial_instrument_id: Option<Uuid>,
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
        financial_instrument_id: Option<Uuid>,
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
            financial_instrument_id,
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

    pub fn reverse_payment(&mut self, payment: &Payment) -> HttpResult<()> {
        if *payment.amount() > self.paid_amount {
            return Err(Box::new(HttpError::bad_request(format!(
                "Cannot reverse payment: amount ({:.2}) exceeds paid amount ({:.2})",
                payment.amount(),
                self.paid_amount
            ))));
        }

        self.paid_amount -= payment.amount();
        self.recalculate_remaining_amount();
        self.recalculate_status();
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    /// Generates installments based on the configured due day from the financial instrument.
    /// Updates the debt's due_date to the last installment date.
    /// Should only be called when has_installments() is true.
    pub fn generate_installments(&mut self, due_day: u32) -> HttpResult<Vec<Installment>> {
        let installment_count = self.installment_count.unwrap_or(0);
        let (base_amount, remainder) = self.calculate_installment_amount(installment_count);

        let mut installments = Vec::new();

        for i in 1..=installment_count {
            let amount = if i == 1 {
                base_amount + remainder
            } else {
                base_amount
            };

            let target_date = self
                .due_date
                .checked_add_months(chrono::Months::new((i - 1) as u32))
                .ok_or_else(|| {
                    Box::new(HttpError::bad_request(format!(
                        "Could not calculate due date for installment {}",
                        i
                    )))
                })?;

            let due_date = date_with_day_or_last(target_date.year(), target_date.month(), due_day);
            installments.push(Installment::new(*self.id(), i, due_date, amount));
        }

        // Update debt's due_date to the last installment's due date
        if let Some(last_installment) = installments.last() {
            self.due_date = *last_installment.due_date();
        }

        Ok(installments)
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
        if self.is_settled() {
            self.status = DebtStatus::Settled;
        } else if self.is_installment() {
            self.status = DebtStatus::Installment;
        } else {
            self.status = DebtStatus::Open;
        }
    }

    fn is_settled(&self) -> bool {
        self.paid_amount >= self.total_amount
    }

    fn is_installment(&self) -> bool {
        self.installment_count.is_some() && self.installment_count.unwrap() > 0
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
    /// Debt is open, not yet paid. A.k.a. "Em aberto"
    #[default]
    Open,
    /// Has installments being paid. A.k.a. "Parcelada"
    Installment,
    /// Fully paid. A.k.a. "Quitada"
    Settled,
}

impl From<String> for DebtStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "OPEN" => DebtStatus::Open,
            "INSTALLMENT" => DebtStatus::Installment,
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
            "OPEN" => DebtStatus::Open,
            "INSTALLMENT" => DebtStatus::Installment,
            "SETTLED" => DebtStatus::Settled,
            // Valores em português (interface do usuário)
            "PENDENTE" => DebtStatus::Open,
            "VENCIDA" => DebtStatus::Installment,
            "PAGO" => DebtStatus::Settled,
            _ => DebtStatus::default(),
        }
    }
}

impl From<DebtStatus> for String {
    fn from(status: DebtStatus) -> Self {
        match status {
            DebtStatus::Open => "OPEN".to_string(),
            DebtStatus::Installment => "INSTALLMENT".to_string(),
            DebtStatus::Settled => "SETTLED".to_string(),
        }
    }
}

impl std::fmt::Display for DebtStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DebtStatus::Open => "OPEN",
            DebtStatus::Installment => "INSTALLMENT",
            DebtStatus::Settled => "SETTLED",
        };
        write!(f, "{}", s)
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
        financial_instrument_id: Option<Uuid>,
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
        financial_instrument_id: Option<Uuid>,
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
    financial_instrument_ids: Option<Vec<Uuid>>,
}

getters!(
    DebtFilters {
        client_id: Option<Uuid>,
        ids: Option<Vec<Uuid>>,
        statuses: Option<Vec<DebtStatus>>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        category_names: Option<Vec<String>>,
        financial_instrument_ids: Option<Vec<Uuid>>,
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

    pub fn with_optional_statuses(mut self, statuses: Option<Vec<DebtStatus>>) -> Self {
        if let Some(s) = statuses {
            self.statuses = Some(s);
        }
        self
    }

    pub fn with_optional_ids(mut self, ids: Option<Vec<Uuid>>) -> Self {
        if let Some(i) = ids {
            self.ids = Some(i);
        }
        self
    }

    pub fn with_optional_start_date(mut self, start_date: Option<NaiveDate>) -> Self {
        if let Some(d) = start_date {
            self.start_date = Some(d);
        }
        self
    }

    pub fn with_optional_end_date(mut self, end_date: Option<NaiveDate>) -> Self {
        if let Some(d) = end_date {
            self.end_date = Some(d);
        }
        self
    }

    pub fn with_optional_category_names(mut self, category_names: Option<Vec<String>>) -> Self {
        if let Some(names) = category_names {
            self.category_names = Some(
                names
                    .into_iter()
                    .map(|name| name.to_uppercase())
                    .collect(),
            );
        }
        self
    }

    pub fn with_financial_instrument_ids(mut self, ids: Vec<Uuid>) -> Self {
        self.financial_instrument_ids = Some(ids);
        self
    }
}
