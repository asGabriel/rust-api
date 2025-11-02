use chrono::{Datelike, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};

use crate::modules::finance_manager::domain::debt::{DebtFilters, DebtStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryFilters {
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub account_identifications: Option<Vec<String>>,
    pub category_names: Option<Vec<String>>,
    pub statuses: Option<Vec<DebtStatus>>,
}

impl SummaryFilters {
    /// Parse command parameters and create filters for the specified month period
    /// Supports:
    /// - No parameters: current month
    /// - MM/YYYY - specific month (e.g., 06/2025)
    /// - d:atual - current month
    /// - d:proximo - next month
    /// - d:anterior - previous month
    /// - c:1 - filter by account identification (single)
    /// - c:2,3,4 - filter by multiple account identifications
    pub fn try_from(parameters: &[String]) -> HttpResult<Self> {
        let mut account_identifications: Option<Vec<String>> = None;
        let mut category_names: Option<Vec<String>> = None;
        let mut statuses: Option<Vec<DebtStatus>> = None;
        let mut date_params = Vec::new();

        // Separate date and account parameters
        for param in parameters {
            if let Some(account_param) = param.strip_prefix("c:") {
                // Parse account identifications (e.g., "1" or "2,3,4")
                let ids: Vec<String> = account_param
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if ids.is_empty() {
                    return Err(Box::new(HttpError::bad_request(
                        "Identificação da conta (c:) requer um número. Exemplo: c:1 ou c:2,3,4",
                    )));
                }

                // Validate that all are numeric
                for id in &ids {
                    id.parse::<i32>().map_err(|_| {
                        Box::new(HttpError::bad_request(format!(
                            "Identificação de conta inválida: '{}'. Use apenas números. Exemplo: c:1 ou c:2,3,4",
                            id
                        )))
                    })?;
                }

                account_identifications = Some(ids);
            } else if let Some(category_param) = param.strip_prefix("cat:") {
                let names: Vec<String> = category_param
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                category_names = Some(names.clone());
                if names.is_empty() {
                    return Err(Box::new(HttpError::bad_request(
                        "Nome da categoria (cat:) requer um nome. Exemplo: cat:investimento",
                    )));
                }
            } else if let Some(status_param) = param.strip_prefix("status:") {
                let parsed_statuses: Vec<DebtStatus> = status_param
                    .split(',')
                    .map(|s| DebtStatus::from(s.trim()))
                    .collect();

                if !parsed_statuses.is_empty() {
                    statuses = Some(parsed_statuses);
                }
            } else {
                date_params.push(param.clone());
            }
        }

        // Parse date parameters
        let date_filters = if date_params.is_empty() {
            // No date parameters: current month
            get_current_month_range()
        } else {
            // Check for d: prefix commands first
            if let Some(first_param) = date_params.first() {
                if let Some(date_param) = first_param.strip_prefix("d:") {
                    parse_date_command(date_param)?
                } else if let Some((month_str, year_str)) = first_param.split_once('/') {
                    parse_mm_yyyy_format(month_str, year_str)?
                } else {
                    return Err(Box::new(HttpError::bad_request(
                        "Parâmetro de data inválido. Use MM/YYYY (ex: 06/2025), d:atual, d:proximo ou d:anterior.",
                    )));
                }
            } else {
                get_current_month_range()
            }
        };

        Ok(SummaryFilters {
            start_date: date_filters.start_date,
            end_date: date_filters.end_date,
            account_identifications,
            category_names,
            statuses,
        })
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

        if let Some(category_names) = &self.category_names {
            filters = filters.with_category_names(category_names.clone());
        }

        if let Some(statuses) = &self.statuses {
            filters = filters.with_statuses(statuses.clone());
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
        account_identifications: None,
        category_names: None,
        statuses: None,
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
        account_identifications: None,
        category_names: None,
        statuses: None,
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
        account_identifications: None,
        category_names: None,
        statuses: None,
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
        account_identifications: None,
        category_names: None,
        statuses: None,
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
