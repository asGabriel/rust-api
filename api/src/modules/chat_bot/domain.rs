use http_error::{HttpError, HttpResult};
use serde::{Deserialize, Serialize};

use crate::modules::chat_bot::domain::{
    debt::NewDebtData, income::NewIncomeData, payment::NewPaymentData, summary::SummaryFilters,
};

pub mod debt;
pub mod formatter;
pub mod income;
pub mod payment;
pub mod summary;
pub mod utils;

/// Trait for command recognition
trait CommandMatcher {
    fn matches(&self, input: &str) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatCommandType {
    Help,
    Summary(SummaryFilters),
    ListIncomes,
    ListAccounts,
    NewIncome(NewIncomeData),
    NewDebt(NewDebtData),
    NewPayment(NewPaymentData),
    Unknown(String),
}

impl ChatCommandType {
    fn try_from_str(command_str: &str, parameters: &[String]) -> HttpResult<Self> {
        let command_str_lower = command_str.to_lowercase();

        // TODO: melhorar esse trecho
        match () {
            _ if HelpCommand.matches(&command_str_lower) => Ok(ChatCommandType::Help),
            _ if SummaryCommand.matches(&command_str_lower) => Ok(ChatCommandType::Summary(
                SummaryFilters::try_from(parameters)?,
            )),
            _ if ListAccountsCommand.matches(&command_str_lower) => {
                Ok(ChatCommandType::ListAccounts)
            }
            _ if ListIncomesCommand.matches(&command_str_lower) => Ok(ChatCommandType::ListIncomes),
            _ if NewIncomeCommand.matches(&command_str_lower) => Ok(ChatCommandType::NewIncome(
                NewIncomeData::try_from(parameters)?,
            )),
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
struct HelpCommand;
struct SummaryCommand;
struct ListAccountsCommand;
struct NewDebtCommand;
struct NewPaymentCommand;
struct ListIncomesCommand;
struct NewIncomeCommand;

impl CommandMatcher for HelpCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(input, "help" | "ajuda" | "?")
    }
}

impl CommandMatcher for SummaryCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(input, "resumo" | "debitos" | "débitos" | "lista-debitos")
    }
}

impl CommandMatcher for ListAccountsCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(input, "contas" | "lista-contas")
    }
}

impl CommandMatcher for NewDebtCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(input, "nova-despesa" | "nova-conta" | "novo" | "despesa")
    }
}

impl CommandMatcher for NewPaymentCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(input, "novo-pagamento" | "pagamento" | "baixa" | "pagar")
    }
}

impl CommandMatcher for ListIncomesCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(input, "receitas" | "lista-receitas")
    }
}

impl CommandMatcher for NewIncomeCommand {
    fn matches(&self, input: &str) -> bool {
        matches!(input, "nova-entrada" | "entrada")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCommand {
    pub command_type: ChatCommandType,
    pub raw_text: String,
    pub parameters: Vec<String>,
}

impl ChatCommand {
    /// Generate a help message with all available commands
    pub fn get_help_message() -> String {
        format!(
            r#"📚 Comandos Disponíveis

📊 Consulta de Débitos
• `resumo` [d:data] [c:numero da conta]
 [d:atual | d:proximo | d:anterior | MM/YYYY] [c:1,2,3]
 exemplo: resumo d:atual c:1,2

💳 Contas
• `contas` - Lista todas as contas cadastradas

➕ Criar Despesa
• `despesa descrição valor c:N cat:categoria [d:data] [p:s]`
  onde: [c:1,2,3], cat:=categoria, d:=data, p:=pago (s=sim, n=não)
  exemplo: despesa mercado 150 c:2 cat:mercado p:n

💰 Registrar Pagamento
• `pagamento identificação [valor] [data]`
  onde: identificação:=número do débito, valor:=valor do pagamento, data:=data do pagamento
  exemplo: pagamento 123 150 2025-01-15
  *obs: valor ausente = valor total do débito | data ausente = data atual

📈 Receitas
• `receitas` - Lista todas as receitas cadastradas

💵 Criar Receita
• `entrada descrição valor c:N [d:data]`
  ex: entrada salario 5000 c:1
  c:=conta, d:=data (usa hoje se não fornecido)

❓ *Ajuda*
• `help`, `ajuda` ou `?` - Mostra esta mensagem
"#
        )
    }

    /// Parse a message and extract the command with detailed error handling.
    /// Returns HttpResult for better error propagation.
    pub fn from_message(text: &str) -> HttpResult<Self> {
        let text = text.trim();

        if text.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Comando inválido: mensagem vazia",
            )));
        }

        // Split by space
        let parts: Vec<String> = text.split_whitespace().map(|s| s.to_string()).collect();

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
    fn test_from_message_valid_new_debt_example1() {
        let result = ChatCommand::from_message("despesa natação 150 c:2");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::NewDebt(data) => {
                assert_eq!(data.description, "natação");
                assert_eq!(data.amount, rust_decimal::Decimal::new(150, 0));
                assert_eq!(data.account_identification, "2");
                assert_eq!(data.is_paid, false);
            }
            _ => panic!("Expected NewDebt command type"),
        }
    }

    #[test]
    fn test_from_message_valid_new_debt_example2() {
        let result = ChatCommand::from_message("despesa mercado 400 c:1 p:s");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::NewDebt(data) => {
                assert_eq!(data.description, "mercado");
                assert_eq!(data.amount, rust_decimal::Decimal::new(400, 0));
                assert_eq!(data.account_identification, "1");
                assert_eq!(data.is_paid, true);
            }
            _ => panic!("Expected NewDebt command type"),
        }
    }

    #[test]
    fn test_from_message_valid_new_debt_with_date() {
        let result = ChatCommand::from_message("despesa almoço 30 c:3 d:2025-01-15");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::NewDebt(data) => {
                assert_eq!(data.description, "almoço");
                assert_eq!(data.amount, rust_decimal::Decimal::new(30, 0));
                assert_eq!(data.account_identification, "3");
                assert!(data.due_date.is_some());
            }
            _ => panic!("Expected NewDebt command type"),
        }
    }

    #[test]
    fn test_from_message_invalid_amount() {
        let result = ChatCommand::from_message("despesa mercado abc c:1");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("Valor"));
    }

    #[test]
    fn test_from_message_missing_amount() {
        let result = ChatCommand::from_message("despesa mercado c:1");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("obrigatório"));
    }

    #[test]
    fn test_from_message_missing_account() {
        let result = ChatCommand::from_message("despesa mercado 100");
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("Conta"));
    }

    #[test]
    fn test_from_message_valid_description_from_number() {
        let result = ChatCommand::from_message("despesa mercado 100 c:1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_from_message_valid_summary() {
        let result = ChatCommand::from_message("resumo");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::Summary(_) => {}
            _ => panic!("Expected Summary command type"),
        }
    }

    #[test]
    fn test_from_message_valid_list_accounts() {
        let result = ChatCommand::from_message("contas");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::ListAccounts => {}
            _ => panic!("Expected ListAccounts command type"),
        }
    }

    #[test]
    fn test_from_message_valid_help() {
        let result = ChatCommand::from_message("help");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::Help => {}
            _ => panic!("Expected Help command type"),
        }
    }

    #[test]
    fn test_from_message_valid_help_ajuda() {
        let result = ChatCommand::from_message("ajuda");
        assert!(result.is_ok());

        let command = result.unwrap();
        match command.command_type {
            ChatCommandType::Help => {}
            _ => panic!("Expected Help command type"),
        }
    }

    #[test]
    fn test_get_help_message() {
        let help_message = ChatCommand::get_help_message();
        assert!(help_message.contains("Comandos Disponíveis"));
        assert!(help_message.contains("resumo"));
        assert!(help_message.contains("contas"));
        assert!(help_message.contains("despesa"));
        assert!(help_message.contains("pagamento"));
        assert!(help_message.contains("receitas"));
        assert!(help_message.contains("entrada"));
    }
}
