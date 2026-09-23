use chrono::{DateTime, Datelike, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{date::date_with_day_or_last, from_row_constructor, getters};
use uuid::Uuid;

use util::DeletedBy;

use crate::modules::finance::domain::payment::PaymentValidator;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Debt {
    id: Uuid,
    client_id: Uuid,
    category: DebtCategory,
    expense_type: ExpenseType,
    list_id: Option<Uuid>,
    identification: String,
    description: String,
    total_amount: Decimal,
    paid_amount: Decimal,
    remaining_amount: Decimal,
    /// `None` na dívida-pai (não tem significado no fluxo de parcelamento).
    /// Sempre `Some` numa dívida comum ou numa filha.
    due_date: Option<NaiveDate>,
    #[serde(default)]
    status: DebtStatus,
    /// Preenchido no pai; copiado nas filhas apenas para exibição ("k/N").
    installment_count: Option<i32>,
    /// `Some` quando esta linha é uma filha (parcela) de outra dívida.
    parent_id: Option<Uuid>,
    /// Ordinal da parcela (1..N) — só preenchido nas filhas.
    installment_number: Option<i32>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_by: Option<DeletedBy>,
}

impl Debt {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        client_id: Uuid,
        description: String,
        total_amount: Decimal,
        due_date: NaiveDate,
        category: Option<DebtCategory>,
        expense_type: Option<ExpenseType>,
        list_id: Option<Uuid>,
        installment_count: Option<i32>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            client_id,
            category: category.unwrap_or_default(),
            expense_type: expense_type.unwrap_or_default(),
            list_id,
            identification: String::new(),
            description,
            total_amount,
            paid_amount: Decimal::ZERO,
            remaining_amount: total_amount,
            due_date: Some(due_date),
            status: DebtStatus::Open,
            installment_count,
            parent_id: None,
            installment_number: None,
            created_at: Utc::now(),
            updated_at: None,
            deleted_by: None,
        }
    }

    pub fn has_installment_count(&self) -> bool {
        self.installment_count.is_some() && self.installment_count.unwrap() > 0
    }

    pub fn is_installment_child(&self) -> bool {
        self.parent_id.is_some()
    }

    /// Top-level debt split into installments. Children also carry a copy of
    /// `installment_count` (for display), hence the `parent_id` check.
    pub fn is_installment_parent(&self) -> bool {
        !self.is_installment_child() && self.has_installment_count()
    }

    pub fn is_settled(&self) -> bool {
        self.status == DebtStatus::Settled
    }

    pub fn exceeds_remaining(&self, amount: Decimal) -> bool {
        amount > self.remaining_amount
    }

    pub fn exceeds_paid(&self, amount: Decimal) -> bool {
        amount > self.paid_amount
    }

    /// Registers `amount` as paid, after validating it against this debt.
    pub fn apply_payment(&mut self, amount: Decimal) -> HttpResult<()> {
        PaymentValidator::new(self).validate(amount)?;

        self.paid_amount += amount;
        self.recompute_balance();
        Ok(())
    }

    /// Reverts a previously applied payment of `amount` (refund).
    pub fn revert_payment(&mut self, amount: Decimal) -> HttpResult<()> {
        if self.is_installment_parent() {
            return Err(Box::new(HttpError::bad_request(
                "An installment parent debt has no payments of its own to revert",
            )));
        }

        if self.exceeds_paid(amount) {
            return Err(Box::new(HttpError::conflict(
                "Refund amount is greater than the debt's paid amount",
            )));
        }

        self.paid_amount -= amount;
        self.recompute_balance();
        Ok(())
    }

    /// Recomputes an installment parent's `paid_amount`, `remaining_amount`
    /// and `status` from its (active) children.
    pub fn sync_with_children(&mut self, children: &[Debt]) {
        self.paid_amount = children.iter().map(|child| child.paid_amount).sum();
        self.recompute_balance();
    }

    fn recompute_balance(&mut self) {
        self.remaining_amount = self.total_amount - self.paid_amount;
        self.status = if self.paid_amount == self.total_amount {
            DebtStatus::Settled
        } else {
            DebtStatus::Open
        };
        self.updated_at = Some(Utc::now());
    }

    /// Gera as N dívidas-filhas a partir desta dívida-pai e zera o
    /// `due_date` do pai (que passa a não ter significado).
    /// Só deve ser chamado quando `has_installment_count()` é `true`.
    pub fn generate_installment_children(&mut self, due_day: u32) -> HttpResult<Vec<Debt>> {
        let installment_count = self.installment_count.unwrap_or(0);
        let (base_amount, remainder) = self.calculate_installment_amount(installment_count);

        let first_due_date = self.due_date.ok_or_else(|| {
            Box::new(HttpError::bad_request(
                "Debt must have a due date to generate installments",
            ))
        })?;

        let mut children = Vec::new();

        for i in 1..=installment_count {
            let amount = if i == installment_count {
                base_amount + remainder
            } else {
                base_amount
            };

            let target_date = first_due_date
                .checked_add_months(chrono::Months::new((i - 1) as u32))
                .ok_or_else(|| {
                    Box::new(HttpError::bad_request(format!(
                        "Could not calculate due date for installment {}",
                        i
                    )))
                })?;

            let due_date = date_with_day_or_last(target_date.year(), target_date.month(), due_day);

            children.push(Debt {
                id: Uuid::new_v4(),
                client_id: self.client_id,
                category: self.category.clone(),
                expense_type: self.expense_type.clone(),
                list_id: self.list_id,
                identification: String::new(),
                description: self.description.clone(),
                total_amount: amount,
                paid_amount: Decimal::ZERO,
                remaining_amount: amount,
                due_date: Some(due_date),
                status: DebtStatus::Open,
                installment_count: Some(installment_count),
                parent_id: Some(self.id),
                installment_number: Some(i),
                created_at: Utc::now(),
                updated_at: None,
                deleted_by: None,
            });
        }

        self.due_date = None;

        Ok(children)
    }

    /// Calculates the amount of the installment and the remainder
    fn calculate_installment_amount(&self, installment_count: i32) -> (Decimal, Decimal) {
        let installment_count = Decimal::from(installment_count);

        let base_amount = (self.total_amount / installment_count).round_dp(2);
        let remainder = self.total_amount - (base_amount * installment_count);
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
    Subscriptions,
    Obligations,
    Purchases,
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
            "SUBSCRIPTIONS" => DebtCategory::Subscriptions,
            "OBLIGATIONS" => DebtCategory::Obligations,
            "PURCHASES" => DebtCategory::Purchases,
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
            DebtCategory::Subscriptions => "SUBSCRIPTIONS".to_string(),
            DebtCategory::Obligations => "OBLIGATIONS".to_string(),
            DebtCategory::Purchases => "PURCHASES".to_string(),
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
    /// Fully paid. A.k.a. "Quitada"
    Settled,
}

impl From<String> for DebtStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "OPEN" => DebtStatus::Open,
            "SETTLED" => DebtStatus::Settled,
            _ => DebtStatus::default(),
        }
    }
}

impl From<DebtStatus> for String {
    fn from(status: DebtStatus) -> Self {
        match status {
            DebtStatus::Open => "OPEN".to_string(),
            DebtStatus::Settled => "SETTLED".to_string(),
        }
    }
}

impl std::fmt::Display for DebtStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            DebtStatus::Open => "OPEN",
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
        list_id: Option<Uuid>,
        identification: String,
        description: String,
        total_amount: Decimal,
        paid_amount: Decimal,
        remaining_amount: Decimal,
        due_date: Option<NaiveDate>,
        status: DebtStatus,
        installment_count: Option<i32>,
        parent_id: Option<Uuid>,
        installment_number: Option<i32>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        deleted_by: Option<DeletedBy>,
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

    pub fn set_list_id(&mut self, list_id: Option<Uuid>) {
        self.list_id = list_id;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_description(&mut self, description: String) {
        self.description = description;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_due_date(&mut self, due_date: NaiveDate) {
        self.due_date = Some(due_date);
        self.updated_at = Some(Utc::now());
    }
}

from_row_constructor! {
    Debt {
        id: Uuid,
        client_id: Uuid,
        category: DebtCategory,
        expense_type: ExpenseType,
        list_id: Option<Uuid>,
        identification: String,
        description: String,
        total_amount: Decimal,
        paid_amount: Decimal,
        remaining_amount: Decimal,
        due_date: Option<NaiveDate>,
        status: DebtStatus,
        installment_count: Option<i32>,
        parent_id: Option<Uuid>,
        installment_number: Option<i32>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        deleted_by: Option<DeletedBy>,
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
    list_id: Option<Uuid>,
    /// `None` = só dívidas de nível-topo (`parent_id IS NULL`, default).
    /// `Some(id)` = lista as filhas (parcelas) da dívida `id`.
    parent_id: Option<Uuid>,
    /// `true` = lista plana que inclui as parcelas junto das dívidas de
    /// nível-topo (sem a restrição `parent_id IS NULL`).
    #[serde(default)]
    include_children: bool,
}

getters!(
    DebtFilters {
        client_id: Option<Uuid>,
        ids: Option<Vec<Uuid>>,
        statuses: Option<Vec<DebtStatus>>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        category_names: Option<Vec<String>>,
        list_id: Option<Uuid>,
        parent_id: Option<Uuid>,
        include_children: bool,
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

    pub fn with_list_id(mut self, list_id: Uuid) -> Self {
        self.list_id = Some(list_id);
        self
    }

    pub fn with_parent_id(mut self, parent_id: Uuid) -> Self {
        self.parent_id = Some(parent_id);
        self
    }

    pub fn with_include_children(mut self, include_children: bool) -> Self {
        self.include_children = include_children;
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
            self.category_names = Some(names.into_iter().map(|name| name.to_uppercase()).collect());
        }
        self
    }

    pub fn with_optional_parent_id(mut self, parent_id: Option<Uuid>) -> Self {
        if let Some(id) = parent_id {
            self.parent_id = Some(id);
        }
        self
    }

    pub fn with_optional_list_id(mut self, list_id: Option<Uuid>) -> Self {
        if let Some(id) = list_id {
            self.list_id = Some(id);
        }
        self
    }
}
