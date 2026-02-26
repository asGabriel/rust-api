use chrono::{DateTime, Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{date::date_with_day_or_last, from_row_constructor, getters};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::debt::{Debt, DebtCategory, ExpenseType},
    handler::debt::use_cases::CreateRecurrenceRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recurrence {
    id: Uuid,
    #[serde(skip_serializing)]
    client_id: Uuid,
    description: String,
    amount: Decimal,
    category: DebtCategory,
    active: bool,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    day_of_month: i32,
    #[serde(default)]
    execution_logs: Vec<RecurrenceExecutionLog>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceExecutionLog {
    run_date: NaiveDate,
    debt_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceFilters {
    client_id: Option<Uuid>,
    active: Option<bool>,
}

impl RecurrenceFilters {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn with_client_id(mut self, client_id: Uuid) -> Self {
        self.client_id = Some(client_id);
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }
}

getters!(
    RecurrenceFilters {
        client_id: Option<Uuid>,
        active: Option<bool>,
    }
);

impl Recurrence {
    pub fn from_request(client_id: Uuid, request: CreateRecurrenceRequest) -> Self {
        Self {
            id: Uuid::new_v4(),
            client_id,
            description: request.description,
            amount: request.amount,
            category: request.category.unwrap_or_default(),
            active: true,
            start_date: request.start_date,
            end_date: request.end_date,
            day_of_month: request.day_of_month,
            execution_logs: Vec::new(),
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    /// Applies partial updates to the recurrence
    pub fn update(
        &mut self,
        description: Option<String>,
        day_of_month: Option<i32>,
        end_date: Option<NaiveDate>,
        active: Option<bool>,
    ) {
        if let Some(description) = description {
            self.description = description;
        }
        if let Some(day_of_month) = day_of_month {
            self.day_of_month = day_of_month;
        }
        if let Some(end_date) = end_date {
            self.end_date = Some(end_date);
        }
        if let Some(active) = active {
            self.active = active;
        }
        self.updated_at = Some(Utc::now());
    }

    /// Checks if this recurrence was already executed for the given year/month
    pub fn was_executed_in_month(&self, year: i32, month: u32) -> bool {
        self.execution_logs
            .iter()
            .any(|log| log.run_date.year() == year && log.run_date.month() == month)
    }

    /// Calculates the due date for a given year/month using the recurrence's day_of_month.
    pub fn calculate_due_date(&self, year: i32, month: u32) -> NaiveDate {
        date_with_day_or_last(year, month, self.day_of_month as u32)
    }

    /// Generates a debt for the given year/month with the calculated due date
    pub fn generate_debt_for_month(&self, year: i32, month: u32) -> Debt {
        let due_date = self.calculate_due_date(year, month);

        Debt::new(
            self.client_id,
            self.description.clone(),
            self.amount,
            None,
            None,
            due_date,
            Some(self.category.clone()),
            Some(ExpenseType::Fixed),
            None,
            None,
            None,
        )
    }

    /// Adds an execution log entry
    pub fn add_execution_log(&mut self, run_date: NaiveDate, debt_id: Uuid) {
        self.execution_logs
            .push(RecurrenceExecutionLog { run_date, debt_id });
        self.updated_at = Some(Utc::now());
    }

    /// Checks if the recurrence is within its valid date range
    pub fn is_within_date_range(&self, date: NaiveDate) -> bool {
        if date < self.start_date {
            return false;
        }
        if let Some(end_date) = self.end_date {
            if date > end_date {
                return false;
            }
        }
        true
    }
}

getters! {
    Recurrence {
        id: Uuid,
        client_id: Uuid,
        description: String,
        amount: Decimal,
        category: DebtCategory,
        active: bool,
        start_date: NaiveDate,
        end_date: Option<NaiveDate>,
        day_of_month: i32,
        execution_logs: Vec<RecurrenceExecutionLog>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

from_row_constructor! {
    Recurrence {
        id: Uuid,
        client_id: Uuid,
        description: String,
        amount: Decimal,
        category: DebtCategory,
        active: bool,
        start_date: NaiveDate,
        end_date: Option<NaiveDate>,
        day_of_month: i32,
        execution_logs: Vec<RecurrenceExecutionLog>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
