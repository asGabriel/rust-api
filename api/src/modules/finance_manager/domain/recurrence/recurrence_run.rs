use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceRun {
    id: Uuid,
    recurrence_id: Uuid,
    debt_id: Uuid,
    run_date: NaiveDate,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}
