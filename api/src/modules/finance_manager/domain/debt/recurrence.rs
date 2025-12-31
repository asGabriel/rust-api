use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

use crate::modules::finance_manager::handler::recurrence::use_cases::CreateRecurrenceRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recurrence {
    id: Uuid,
    account_id: Uuid,
    description: String,
    amount: Decimal,
    active: bool,
    start_date: NaiveDate,
    end_date: Option<NaiveDate>,
    day_of_month: i32,
    next_run_date: NaiveDate,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RecurrenceFilters {
    next_run_date: Option<NaiveDate>,
    active: Option<bool>,
}

impl RecurrenceFilters {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
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
        next_run_date: Option<NaiveDate>,
        active: Option<bool>,
    }
);

impl Recurrence {
    pub fn from_request(request: CreateRecurrenceRequest, account_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            account_id,
            description: request.description,
            amount: request.amount,
            active: true,
            start_date: request.start_date,
            end_date: request.end_date,
            day_of_month: request.day_of_month,
            next_run_date: request.start_date,
            created_at: Utc::now(),
            updated_at: None,
        }
    }
}

getters! {
    Recurrence {
        id: Uuid,
        account_id: Uuid,
        description: String,
        amount: Decimal,
        active: bool,
        start_date: NaiveDate,
        end_date: Option<NaiveDate>,
        day_of_month: i32,
        next_run_date: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

from_row_constructor! {
    Recurrence {
        id: Uuid,
        account_id: Uuid,
        description: String,
        amount: Decimal,
        active: bool,
        start_date: NaiveDate,
        end_date: Option<NaiveDate>,
        day_of_month: i32,
        next_run_date: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
