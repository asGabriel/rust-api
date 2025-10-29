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
    /// - d:proximo - next month
    /// - d:anterior - previous month
    /// - d:MM-YY or d:MMM/YY - specific month (e.g., d:06-25 or d:jun/25)
    pub fn try_from(parameters: &[String]) -> HttpResult<Self> {
        if parameters.is_empty() {
            // No parameters: current month
            return Ok(get_current_month_range());
        }

        // Look for d: parameter
        for param in parameters {
            if let Some(date_param) = param.strip_prefix("d:") {
                return parse_month_filter(date_param);
            }
        }

        // If parameters exist but no d: found, return error
        Err(Box::new(HttpError::bad_request(
            "Parâmetro de data inválido. Use 'd:proximo', 'd:anterior' ou 'd:MM-YY' (ex: d:06-25)",
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

/// Parse a month filter parameter
fn parse_month_filter(param: &str) -> HttpResult<SummaryFilters> {
    let param_lower = param.to_lowercase();

    match param_lower.as_str() {
        "proximo" | "próximo" => Ok(get_next_month_range()),
        "anterior" => Ok(get_previous_month_range()),
        _ => {
            // Try to parse as MM-YY or MMM/YY format
            parse_specific_month(param)
        }
    }
}

/// Parse specific month formats: MM-YY or MMM/YY (e.g., 06-25 or jun/25)
fn parse_specific_month(param: &str) -> HttpResult<SummaryFilters> {
    // Try MM-YY format (e.g., 06-25)
    if let Some((month_str, year_str)) = param.split_once('-') {
        let month: u32 = month_str.parse().map_err(|_| {
            Box::new(HttpError::bad_request(format!(
                "Mês inválido no formato 'd:MM-YY'. Exemplo: d:06-25"
            )))
        })?;

        let year_short: i32 = year_str.parse().map_err(|_| {
            Box::new(HttpError::bad_request(format!(
                "Ano inválido no formato 'd:MM-YY'. Exemplo: d:06-25"
            )))
        })?;

        // Convert 2-digit year to 4-digit (assume 20xx for 00-99)
        let year = if year_short < 100 {
            if year_short < 50 {
                2000 + year_short
            } else {
                1900 + year_short
            }
        } else {
            year_short
        };

        if month < 1 || month > 12 {
            return Err(Box::new(HttpError::bad_request(format!(
                "Mês inválido: {}. Deve ser entre 1 e 12",
                month
            ))));
        }

        let (start, end) = get_month_range(year, month);
        return Ok(SummaryFilters {
            start_date: Some(start),
            end_date: Some(end),
        });
    }

    // Try MMM/YY format (e.g., jun/25)
    if let Some((month_str, year_str)) = param.split_once('/') {
        let month = parse_month_name(month_str)?;
        let year_short: i32 = year_str.parse().map_err(|_| {
            Box::new(HttpError::bad_request(format!(
                "Ano inválido no formato 'd:MMM/YY'. Exemplo: d:jun/25"
            )))
        })?;

        let year = if year_short < 100 {
            if year_short < 50 {
                2000 + year_short
            } else {
                1900 + year_short
            }
        } else {
            year_short
        };

        let (start, end) = get_month_range(year, month);
        return Ok(SummaryFilters {
            start_date: Some(start),
            end_date: Some(end),
        });
    }

    Err(Box::new(HttpError::bad_request(format!(
        "Formato de data inválido: 'd:{}'. Use 'd:proximo', 'd:anterior', 'd:MM-YY' (ex: d:06-25) ou 'd:MMM/YY' (ex: d:jun/25)",
        param
    ))))
}

/// Parse month name in Portuguese (jan, fev, mar, etc.)
fn parse_month_name(month_str: &str) -> HttpResult<u32> {
    let month_lower = month_str.to_lowercase();
    let month = match month_lower.as_str() {
        "jan" | "janeiro" => 1,
        "fev" | "fevereiro" => 2,
        "mar" | "marco" | "março" => 3,
        "abr" | "abril" => 4,
        "mai" | "maio" => 5,
        "jun" | "junho" => 6,
        "jul" | "julho" => 7,
        "ago" | "agosto" => 8,
        "set" | "setembro" => 9,
        "out" | "outubro" => 10,
        "nov" | "novembro" => 11,
        "dez" | "dezembro" => 12,
        _ => {
            return Err(Box::new(HttpError::bad_request(format!(
                "Nome de mês inválido: '{}'. Use abreviações como 'jan', 'fev', 'mar', etc.",
                month_str
            ))));
        }
    };
    Ok(month)
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
