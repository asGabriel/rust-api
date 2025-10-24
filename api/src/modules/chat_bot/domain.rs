use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};

use crate::modules::chat_bot::domain::{debt::NewDebtData, payment::NewPaymentData};

pub mod debt;
pub mod payment;
pub mod formatter;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatCommandType {
    ListDebts,
    ListAccounts,
    NewDebt(NewDebtData),
    NewPayment(NewPaymentData),
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCommand {
    pub command_type: ChatCommandType,
    pub raw_text: String,
    pub parameters: Vec<String>,
}

impl ChatCommand {
    /// Parse a message and extract the command.
    pub fn from_message(text: &str) -> Option<Self> {
        let text = text.trim();

        if !text.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = text[1..].split("!").collect();
        if parts.is_empty() {
            return None;
        }

        let command_str = parts[0].to_lowercase();
        let parameters: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        let command_type = match command_str.as_str() {
            "debitos" | "debts" | "débitos" => ChatCommandType::ListDebts,
            "novo" | "new" => match NewDebtData::try_from(&parameters) {
                Ok(data) => ChatCommandType::NewDebt(data),
                Err(_) => return None,
            },
            "contas" | "accounts" => ChatCommandType::ListAccounts,
            _ => ChatCommandType::Unknown(command_str),
        };

        Some(ChatCommand {
            command_type,
            raw_text: text.to_string(),
            parameters,
        })
    }

    /// Parse a message and extract the command with detailed error handling.
    /// Returns HttpResult for better error propagation.
    pub fn from_message_with_errors(text: &str) -> HttpResult<Self> {
        let text = text.trim();

        if !text.starts_with('/') {
            return Err(Box::new(HttpError::bad_request(
                "Comando deve começar com '/'",
            )));
        }

        let parts: Vec<&str> = text[1..].split("!").collect();
        if parts.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Comando inválido: formato esperado /comando!param1!param2",
            )));
        }

        let command_str = parts[0].to_lowercase();
        let parameters: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        let command_type = match command_str.as_str() {
            "debitos" | "debts" | "débitos" => ChatCommandType::ListDebts,
            "novo" | "new" => {
                let data = NewDebtData::try_from(&parameters)?;
                ChatCommandType::NewDebt(data)
            }
            _ => ChatCommandType::Unknown(command_str),
        };

        Ok(ChatCommand {
            command_type,
            raw_text: text.to_string(),
            parameters,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_message_with_errors_valid_new_debt() {
        let result = ChatCommand::from_message_with_errors("/novo!mercado da semana!500!ABCD");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::NewDebt(data) => {
                assert_eq!(data.description, "mercado da semana");
                assert_eq!(data.amount, rust_decimal::Decimal::new(500, 0));
                assert_eq!(data.account_identification, "ABCD");
            }
            _ => panic!("Expected NewDebt command type"),
        }
    }

    #[test]
    fn test_from_message_with_errors_invalid_new_debt() {
        let result = ChatCommand::from_message_with_errors("/novo!mercado da semana!abc!ABCD");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("Valor inválido"));
    }

    #[test]
    fn test_from_message_with_errors_empty_description() {
        let result = ChatCommand::from_message_with_errors("/novo!!500!ABCD");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("Descrição não pode estar vazia"));
    }

    #[test]
    fn test_from_message_with_errors_insufficient_params() {
        let result = ChatCommand::from_message_with_errors("/novo!descrição!100");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("3 parâmetros"));
    }

    #[test]
    fn test_from_message_with_errors_invalid_command_format() {
        let result = ChatCommand::from_message_with_errors("hello world");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("deve começar com '/'"));
    }

    #[test]
    fn test_from_message_with_errors_invalid_account_id() {
        let result = ChatCommand::from_message_with_errors("/novo!descrição!100!ABC");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("exatamente 4 caracteres"));
    }
}
