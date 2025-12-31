use chrono::{Datelike, NaiveDate, Utc};
use http_error::HttpResult;
use rust_decimal::Decimal;

/// Trait for formatting data for chat display
pub trait ChatFormatter {
    /// Formats a single item for chat display
    fn format_for_chat(&self) -> String;

    /// Formats a list of items for chat display
    fn format_list_for_chat(items: &[Self]) -> String
    where
        Self: Sized;
}

/// Generic utilities for chat formatting
pub struct ChatFormatterUtils;

impl ChatFormatterUtils {
    /// Formats decimal values with 2 decimal places
    pub fn format_decimal(decimal: &Decimal) -> String {
        format!("{:.2}", decimal)
    }

    // TODO: move this for the correct place
    /// Formats debt status with emoji
    pub fn format_debt_status(
        status: &crate::modules::finance_manager::domain::debt::DebtStatus,
    ) -> String {
        match status {
            crate::modules::finance_manager::domain::debt::DebtStatus::Unpaid => "🔴 Unpaid",
            crate::modules::finance_manager::domain::debt::DebtStatus::PartiallyPaid => {
                "🟡 Partially Paid"
            }
            crate::modules::finance_manager::domain::debt::DebtStatus::Settled => "🟢 Settled",
        }
        .to_string()
    }

    /// Formats date as DD/MM/YYYY
    pub fn format_date(date: &chrono::NaiveDate) -> String {
        date.format("%d/%m/%Y").to_string()
    }

    /// Formats datetime as DD/MM/YYYY HH:MM
    pub fn format_datetime(datetime: &chrono::DateTime<chrono::Utc>) -> String {
        datetime.format("%d/%m/%Y %H:%M").to_string()
    }

    /// Returns separator line
    pub fn separator() -> String {
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string()
    }

    /// Formats value as currency (R$)
    pub fn format_currency(value: &Decimal) -> String {
        format!("R$ {}", Self::format_decimal(value))
    }

    /// Formats items as numbered list
    pub fn format_numbered_list<T>(items: &[T], formatter: impl Fn(&T) -> String) -> String {
        if items.is_empty() {
            return "No items found".to_string();
        }

        items
            .iter()
            .enumerate()
            .map(|(i, item)| format!("{}. {}", i + 1, formatter(item)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Parse a friendly date string into NaiveDate
    /// Supports:
    /// - "dd/mm/yyyy" or "dd/mm" (assumes current year)
    /// - "dd-mm-yyyy" or "dd-mm"
    /// - "dd.mm.yyyy" or "dd.mm"
    /// - "hoje" or "today"
    /// - "amanhã" or "tomorrow"
    /// - "+n" or "em-n-dias" (n days from today)
    pub fn parse_friendly_date(input: &str) -> HttpResult<NaiveDate> {
        use http_error::HttpError;

        let input = input.trim().to_lowercase();

        // Special keywords
        match input.as_str() {
            "hoje" | "today" => return Ok(Utc::now().date_naive()),
            "amanhã" | "tomorrow" => {
                return Ok(Utc::now().date_naive() + chrono::Duration::days(1));
            }
            _ => {}
        }

        // "+n" or "em-n-dias" format
        if let Some(stripped) = input.strip_prefix('+') {
            if let Ok(days) = stripped.trim().parse::<i64>() {
                return Ok(Utc::now().date_naive() + chrono::Duration::days(days));
            }
        }

        if input.starts_with("em-") && input.ends_with("-dias") {
            let days_str = &input[3..input.len() - 5];
            if let Ok(days) = days_str.parse::<i64>() {
                return Ok(Utc::now().date_naive() + chrono::Duration::days(days));
            }
        }

        // Try parsing as dd/mm/yyyy, dd-mm-yyyy, or dd.mm.yyyy
        let parts: Vec<&str> = input.split(['/', '-', '.']).collect();
        if parts.len() >= 2 {
            let day: u32 = parts[0].parse().map_err(|_| {
                Box::new(HttpError::bad_request(format!(
                    "Data inválida: dia '{}'",
                    parts[0]
                )))
            })?;
            let month: u32 = parts[1].parse().map_err(|_| {
                Box::new(HttpError::bad_request(format!(
                    "Data inválida: mês '{}'",
                    parts[1]
                )))
            })?;
            let year = if parts.len() >= 3 {
                parts[2].parse().map_err(|_| {
                    Box::new(HttpError::bad_request(format!(
                        "Data inválida: ano '{}'",
                        parts[2]
                    )))
                })?
            } else {
                Utc::now().year()
            };

            if let Some(date) = NaiveDate::from_ymd_opt(year, month, day) {
                return Ok(date);
            }
        }

        // Try parsing as ISO format (yyyy-mm-dd)
        if let Ok(date) = input.parse::<NaiveDate>() {
            return Ok(date);
        }

        Err(Box::new(HttpError::bad_request(format!(
            "Data inválida: '{}'. Formatos aceitos: dd/mm/yyyy, dd/mm, hoje, amanhã, +n dias",
            input
        ))))
    }
}
