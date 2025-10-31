use chrono::{Datelike, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};

use crate::modules::finance_manager::domain::debt::DebtFilters;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryFilters {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}

impl SummaryFilters {
    /// Parse command parameters and create filters for the specified month period
    /// Supports:
    /// - No parameters: current month
    /// - MM/YYYY - specific month (e.g., 06/2025)
    /// - d:atual - current month
    /// - d:proximo - next month
    /// - d:anterior - previous month
    pub fn try_from(parameters: &[String]) -> HttpResult<Self> {
        if parameters.is_empty() {
            // No parameters: current month
            return Ok(get_current_month_range());
        }

        // Check for d: prefix commands first
        if let Some(first_param) = parameters.first() {
            if let Some(date_param) = first_param.strip_prefix("d:") {
                return parse_date_command(date_param);
            }
        }

        // Check for MM/YYYY format (e.g., 06/2025)
        if let Some(first_param) = parameters.first() {
            if let Some((month_str, year_str)) = first_param.split_once('/') {
                return parse_mm_yyyy_format(month_str, year_str);
            }
        }

        // If parameter format is not recognized, return error
        Err(Box::new(HttpError::bad_request(
            "Parâmetro de data inválido. Use MM/YYYY (ex: 06/2025), d:atual, d:proximo ou d:anterior.",
        )))
    }

    /// Convert SummaryFilters to DebtFilters for querying
    pub fn to_debt_filters(&self) -> DebtFilters {
        let mut filters = DebtFilters::default();

        if let Some(start) = self.start_date {
            filters = filters.with_start_date(start);
        }

        if let Some(end) = self.end_date {
            filters = filters.with_end_date(end);
        }

        filters
    }
}

/// Get the current month date range (first day to last day)
fn get_current_month_range() -> SummaryFilters {
    let now = Utc::now().date_naive();
    let (start, end) = get_month_range(now.year(), now.month());
    SummaryFilters {
        start_date: Some(start),
        end_date: Some(end),
    }
}

/// Get the next month date range
fn get_next_month_range() -> SummaryFilters {
    let now = Utc::now().date_naive();
    let next_month = if now.month() == 12 {
        (now.year() + 1, 1)
    } else {
        (now.year(), now.month() + 1)
    };
    let (start, end) = get_month_range(next_month.0, next_month.1);
    SummaryFilters {
        start_date: Some(start),
        end_date: Some(end),
    }
}

/// Get the previous month date range
fn get_previous_month_range() -> SummaryFilters {
    let now = Utc::now().date_naive();
    let prev_month = if now.month() == 1 {
        (now.year() - 1, 12)
    } else {
        (now.year(), now.month() - 1)
    };
    let (start, end) = get_month_range(prev_month.0, prev_month.1);
    SummaryFilters {
        start_date: Some(start),
        end_date: Some(end),
    }
}

/// Parse date command (d:atual, d:proximo, d:anterior)
fn parse_date_command(param: &str) -> HttpResult<SummaryFilters> {
    let param_lower = param.to_lowercase();
    match param_lower.as_str() {
        "atual" => Ok(get_current_month_range()),
        "proximo" | "próximo" => Ok(get_next_month_range()),
        "anterior" => Ok(get_previous_month_range()),
        _ => Err(Box::new(HttpError::bad_request(format!(
            "Comando inválido: 'd:{}'. Use d:atual, d:proximo ou d:anterior.",
            param
        )))),
    }
}

/// Parse MM/YYYY format (e.g., 06/2025)
fn parse_mm_yyyy_format(month_str: &str, year_str: &str) -> HttpResult<SummaryFilters> {
    let month: u32 = month_str.parse().map_err(|_| {
        Box::new(HttpError::bad_request(format!(
            "Mês inválido no formato MM/YYYY. Use um número de 01 a 12. Exemplo: 06/2025"
        )))
    })?;

    let year: i32 = year_str.parse().map_err(|_| {
        Box::new(HttpError::bad_request(format!(
            "Ano inválido no formato MM/YYYY. Use um ano válido (ex: 2025). Exemplo: 06/2025"
        )))
    })?;

    if month < 1 || month > 12 {
        return Err(Box::new(HttpError::bad_request(format!(
            "Mês inválido: {}. Deve ser entre 1 e 12",
            month
        ))));
    }

    if year < 1900 || year > 2100 {
        return Err(Box::new(HttpError::bad_request(format!(
            "Ano inválido: {}. Deve ser entre 1900 e 2100",
            year
        ))));
    }

    let (start, end) = get_month_range(year, month);
    Ok(SummaryFilters {
        start_date: Some(start),
        end_date: Some(end),
    })
}

/// Get the first and last day of a given month
fn get_month_range(year: i32, month: u32) -> (NaiveDate, NaiveDate) {
    let start = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    // Get the last day of the month by using the number of days in the month
    let days_in_month = if month == 12 {
        // December: calculate using next year January minus 1 day
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
            .unwrap()
            .signed_duration_since(start)
            .num_days() as u32
    } else {
        // Other months: calculate using next month first day minus 1 day
        NaiveDate::from_ymd_opt(year, month + 1, 1)
            .unwrap()
            .signed_duration_since(start)
            .num_days() as u32
    };

    let end = NaiveDate::from_ymd_opt(year, month, days_in_month).unwrap();
    (start, end)
}
