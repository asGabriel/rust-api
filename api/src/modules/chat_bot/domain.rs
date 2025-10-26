use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};

use crate::modules::chat_bot::domain::{debt::NewDebtData, payment::NewPaymentData};

pub mod debt;
pub mod formatter;
pub mod payment;

/// Trait for command recognition
trait CommandMatcher {
    fn matches(&self, input: &str) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatCommandType {
    Summary,
    ListAccounts,
    NewDebt(NewDebtData),
    NewPayment(NewPaymentData),
    Unknown(String),
}

impl ChatCommandType {
    fn try_from_str(command_str: &str, parameters: &[String]) -> HttpResult<Self> {
        let command_str_lower = command_str.to_lowercase();

        match () {
            _ if SummaryCommand.matches(&command_str_lower) => Ok(ChatCommandType::Summary),
            _ if ListAccountsCommand.matches(&command_str_lower) => {
                Ok(ChatCommandType::ListAccounts)
            }
            _ if NewDebtCommand.matches(&command_str_lower) => {
                Ok(ChatCommandType::NewDebt(NewDebtData::try_from(parameters)?))
            }
            _ if NewPaymentCommand.matches(&command_str_lower) => Ok(ChatCommandType::NewPayment(
                NewPaymentData::try_from(parameters)?,
            )),
            _ => Err(Box::new(HttpError::bad_request(format!(
                "Comando desconhecido: '{}'",
                command_str_lower
            )))),
        }
    }
}

// Command variants
struct SummaryCommand;
struct ListAccountsCommand;
struct NewDebtCommand;
struct NewPaymentCommand;

impl CommandMatcher for SummaryCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(
            input,
            "resumo" | "summary" | "debitos" | "débitos" | "lista-debitos"
        )
    }
}

impl CommandMatcher for ListAccountsCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(input, "contas" | "accounts" | "lista-contas")
    }
}

impl CommandMatcher for NewDebtCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(
            input,
            "nova-despesa" | "nova-conta" | "novo" | "despesa" | "new-debt" | "new"
        )
    }
}

impl CommandMatcher for NewPaymentCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(
            input,
            "novo-pagamento" | "novo-payment" | "pagamento" | "payment" | "baixa" | "pagar"
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCommand {
    pub command_type: ChatCommandType,
    pub raw_text: String,
    pub parameters: Vec<String>,
}

impl ChatCommand {
    /// Parse a message and extract the command.
    /// Returns None if the message is not a command or parsing fails.
    pub fn from_message(text: &str) -> Option<Self> {
        let text = text.trim();

        // Split by '!' and remove spaces around
        let parts: Vec<String> = text.split('!').map(|s| s.trim().to_string()).collect();

        if parts.is_empty() {
            return None;
        }

        let command_str = parts[0].to_lowercase();
        let parameters: Vec<String> = parts[1..].to_vec();

        // Silently fail on errors
        let command_type = ChatCommandType::try_from_str(&command_str, &parameters).ok()?;

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

        // Split by '!' and remove spaces around
        let parts: Vec<String> = text.split('!').map(|s| s.trim().to_string()).collect();

        if parts.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Comando inválido: formato esperado comando!param1!param2",
            )));
        }

        let command_str = parts[0].to_lowercase();
        let parameters: Vec<String> = parts[1..].to_vec();

        let command_type = ChatCommandType::try_from_str(&command_str, &parameters)?;

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
    fn test_from_message_with_errors_valid_new_debt_nova_conta() {
        let result = ChatCommand::from_message_with_errors("nova-conta!mercado da semana!500!ABCD");
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
    fn test_from_message_with_errors_valid_new_debt_nova_despesa() {
        let result = ChatCommand::from_message_with_errors("nova-despesa!mercado!500!ABCD");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::NewDebt(data) => {
                assert_eq!(data.description, "mercado");
                assert_eq!(data.amount, rust_decimal::Decimal::new(500, 0));
                assert_eq!(data.account_identification, "ABCD");
            }
            _ => panic!("Expected NewDebt command type"),
        }
    }

    #[test]
    fn test_from_message_with_errors_invalid_new_debt() {
        let result = ChatCommand::from_message_with_errors("nova-conta!mercado da semana!abc!ABCD");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("Valor inválido"));
    }

    #[test]
    fn test_from_message_with_errors_empty_description() {
        let result = ChatCommand::from_message_with_errors("nova-conta!!500!ABCD");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("Descrição não pode estar vazia"));
    }

    #[test]
    fn test_from_message_with_errors_insufficient_params() {
        let result = ChatCommand::from_message_with_errors("nova-conta!descrição!100");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("3 parâmetros"));
    }

    #[test]
    fn test_from_message_with_errors_spaces_around_exclamation() {
        let result = ChatCommand::from_message_with_errors("nova-conta !mercado!500!ABCD");
        assert!(result.is_ok()); // Should handle spaces correctly
    }

    #[test]
    fn test_from_message_with_errors_invalid_account_id() {
        let result = ChatCommand::from_message_with_errors("nova-conta!descrição!100!ABC");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("4 caracteres"));
    }

    #[test]
    fn test_from_message_valid_summary() {
        let result = ChatCommand::from_message("resumo");
        assert!(result.is_some());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::Summary => {}
            _ => panic!("Expected Summary command type"),
        }
    }

    #[test]
    fn test_from_message_valid_list_accounts() {
        let result = ChatCommand::from_message("contas");
        assert!(result.is_some());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::ListAccounts => {}
            _ => panic!("Expected ListAccounts command type"),
        }
    }
}
