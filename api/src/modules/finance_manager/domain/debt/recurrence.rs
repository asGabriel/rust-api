use chrono::{DateTime, Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{date::date_with_day_or_last, from_row_constructor, getters};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::{
        debt::{Debt, ExpenseType},
        financial_instrument::FinancialInstrument,
    },
    handler::debt::use_cases::CreateRecurrenceRequest,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recurrence {
    id: Uuid,
    client_id: Uuid,
    account_id: Option<Uuid>,
    description: String,
    amount: Decimal,
    active: bool,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    day_of_month: i32,
    next_run_date: NaiveDate,
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
    next_run_date: Option<NaiveDate>,
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

    pub fn with_next_run_date(mut self, next_run_date: NaiveDate) -> Self {
        self.next_run_date = Some(next_run_date);
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
        next_run_date: Option<NaiveDate>,
        active: Option<bool>,
    }
);

impl Recurrence {
    pub fn from_request(
        client_id: Uuid,
        request: CreateRecurrenceRequest,
        financial_instrument: Option<FinancialInstrument>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            client_id,
            account_id: financial_instrument.map(|instrument| *instrument.id()),
            description: request.description,
            amount: request.amount,
            active: true,
            start_date: request.start_date,
            end_date: request.end_date,
            day_of_month: request.day_of_month,
            next_run_date: request.start_date,
            execution_logs: Vec::new(),
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    /// Checks if this recurrence was already executed for the given year/month
    pub fn was_executed_in_month(&self, year: i32, month: u32) -> bool {
        self.execution_logs
            .iter()
            .any(|log| log.run_date.year() == year && log.run_date.month() == month)
    }

    /// Calculates the due date for a given year/month.
    /// If a financial instrument is provided and has a configured due date, use it.
    /// Otherwise, use the recurrence's day_of_month.
    pub fn calculate_due_date(
        &self,
        financial_instrument: Option<&FinancialInstrument>,
        year: i32,
        month: u32,
    ) -> NaiveDate {
        let day = financial_instrument
            .and_then(|fi| fi.configuration().default_due_date)
            .unwrap_or(self.day_of_month as u32);

        date_with_day_or_last(year, month, day)
    }

    /// Generates a debt for the given year/month with the calculated due date
    pub fn generate_debt_for_month(
        &self,
        financial_instrument: Option<&FinancialInstrument>,
        year: i32,
        month: u32,
    ) -> Debt {
        let due_date = self.calculate_due_date(financial_instrument, year, month);

        Debt::new(
            self.client_id,
            self.description.clone(),
            self.amount,
            None,
            None,
            due_date,
            None,
            Some(ExpenseType::Fixed),
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
        account_id: Option<Uuid>,
        description: String,
        amount: Decimal,
        active: bool,
        start_date: NaiveDate,
        end_date: Option<NaiveDate>,
        day_of_month: i32,
        next_run_date: NaiveDate,
        execution_logs: Vec<RecurrenceExecutionLog>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

from_row_constructor! {
    Recurrence {
        id: Uuid,
        client_id: Uuid,
        account_id: Option<Uuid>,
        description: String,
        amount: Decimal,
        active: bool,
        start_date: NaiveDate,
        end_date: Option<NaiveDate>,
        day_of_month: i32,
        next_run_date: NaiveDate,
        execution_logs: Vec<RecurrenceExecutionLog>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
