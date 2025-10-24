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
}
