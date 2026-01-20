use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InstrumentConfiguration {
    pub default_due_date: Option<u32>,
}

impl InstrumentConfiguration {
    pub fn default_due_date(&self) -> Option<NaiveDate> {
        let now = Utc::now().date_naive();
        self.default_due_date.map(|days| {
            if now.day() > days {
                NaiveDate::from_ymd_opt(now.year(), now.month() + 1, days).unwrap()
            } else {
                NaiveDate::from_ymd_opt(now.year(), now.month(), days).unwrap()
            }
        })
    }
}
