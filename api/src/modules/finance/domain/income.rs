use chrono::{DateTime, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters, DeletedBy};
use uuid::Uuid;

use crate::modules::finance::domain::money::MoneyExt;

/// Money that actually came in. There is no expected/pending income: an
/// income only exists once it has been received.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Income {
    id: Uuid,
    client_id: Uuid,
    category: IncomeCategory,
    description: String,
    amount: Decimal,
    /// Day the money was received; the month it counts in comes from here.
    received_date: NaiveDate,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_by: Option<DeletedBy>,
}

impl Income {
    pub fn new(
        client_id: Uuid,
        description: String,
        amount: Decimal,
        received_date: NaiveDate,
        category: Option<IncomeCategory>,
    ) -> HttpResult<Self> {
        Self::validate_amount(amount)?;

        Ok(Self {
            id: Uuid::new_v4(),
            client_id,
            category: category.unwrap_or_default(),
            description,
            amount,
            received_date,
            created_at: Utc::now(),
            updated_at: None,
            deleted_by: None,
        })
    }

    pub fn set_category(&mut self, category: IncomeCategory) {
        self.category = category;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_description(&mut self, description: String) {
        self.description = description;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_amount(&mut self, amount: Decimal) -> HttpResult<()> {
        Self::validate_amount(amount)?;

        self.amount = amount;
        self.updated_at = Some(Utc::now());
        Ok(())
    }

    pub fn set_received_date(&mut self, received_date: NaiveDate) {
        self.received_date = received_date;
        self.updated_at = Some(Utc::now());
    }

    fn validate_amount(amount: Decimal) -> HttpResult<()> {
        if amount <= Decimal::ZERO {
            return Err(Box::new(HttpError::bad_request(
                "Income amount must be greater than zero",
            )));
        }

        if amount.exceeds_money_scale() {
            return Err(Box::new(HttpError::bad_request(
                "Income amount must have at most 2 decimal places",
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IncomeCategory {
    #[default]
    Unknown,
    Salary,
    Freelance,
    Investment,
    Refund,
}

impl From<String> for IncomeCategory {
    fn from(s: String) -> Self {
        match s.as_str() {
            "SALARY" => IncomeCategory::Salary,
            "FREELANCE" => IncomeCategory::Freelance,
            "INVESTMENT" => IncomeCategory::Investment,
            "REFUND" => IncomeCategory::Refund,
            _ => IncomeCategory::Unknown,
        }
    }
}

impl From<IncomeCategory> for String {
    fn from(category: IncomeCategory) -> Self {
        match category {
            IncomeCategory::Salary => "SALARY".to_string(),
            IncomeCategory::Freelance => "FREELANCE".to_string(),
            IncomeCategory::Investment => "INVESTMENT".to_string(),
            IncomeCategory::Refund => "REFUND".to_string(),
            IncomeCategory::Unknown => "UNKNOWN".to_string(),
        }
    }
}

getters! {
    Income {
        id: Uuid,
        client_id: Uuid,
        category: IncomeCategory,
        description: String,
        amount: Decimal,
        received_date: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        deleted_by: Option<DeletedBy>,
    }
}

from_row_constructor! {
    Income {
        id: Uuid,
        client_id: Uuid,
        category: IncomeCategory,
        description: String,
        amount: Decimal,
        received_date: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        deleted_by: Option<DeletedBy>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IncomeFilters {
    client_id: Option<Uuid>,
    categories: Option<Vec<IncomeCategory>>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

getters!(
    IncomeFilters {
        client_id: Option<Uuid>,
        categories: Option<Vec<IncomeCategory>>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    }
);

impl IncomeFilters {
    pub fn new(client_id: Uuid) -> Self {
        Self {
            client_id: Some(client_id),
            ..Default::default()
        }
    }

    pub fn with_optional_categories(mut self, categories: Option<Vec<IncomeCategory>>) -> Self {
        if let Some(c) = categories {
            self.categories = Some(c);
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(value: &str) -> Decimal {
        value.parse().unwrap()
    }

    fn received_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 5).unwrap()
    }

    fn new_income(amount: &str) -> HttpResult<Income> {
        Income::new(
            Uuid::nil(),
            "income".to_string(),
            d(amount),
            received_date(),
            None,
        )
    }

    #[test]
    fn new_defaults_category_to_unknown() {
        let income = new_income("8500").unwrap();

        assert_eq!(income.category(), &IncomeCategory::Unknown);
    }

    #[test]
    fn new_rejects_non_positive_amount() {
        assert!(new_income("0").is_err());
        assert!(new_income("-10").is_err());
    }

    #[test]
    fn new_rejects_more_than_two_decimal_places() {
        assert!(new_income("10.005").is_err());
        assert!(new_income("10.500").is_ok());
    }

    #[test]
    fn set_amount_keeps_current_value_when_invalid() {
        let mut income = new_income("100").unwrap();

        assert!(income.set_amount(d("0")).is_err());
        assert!(income.set_amount(d("1.234")).is_err());
        assert_eq!(income.amount(), &d("100"));
    }

    #[test]
    fn category_round_trips_through_string() {
        for category in [
            IncomeCategory::Unknown,
            IncomeCategory::Salary,
            IncomeCategory::Freelance,
            IncomeCategory::Investment,
            IncomeCategory::Refund,
        ] {
            assert_eq!(
                IncomeCategory::from(String::from(category.clone())),
                category
            );
        }
    }
}
