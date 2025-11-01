use chrono::{Datelike, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AccountConfiguration {
    pub default_due_date: Option<u32>,
}

impl AccountConfiguration {
    pub fn default_due_date(&self) -> Option<NaiveDate> {
        let now = Utc::now().date_naive();
        // se o dia for maior que o "default_due_date" tem que retornar o dia no mes seguinte
        self.default_due_date.map(|days| {
            if now.day() > days {
                NaiveDate::from_ymd_opt(now.year(), now.month() + 1, days).unwrap()
            } else {
                NaiveDate::from_ymd_opt(now.year(), now.month(), days).unwrap()
            }
        })
    }
}
