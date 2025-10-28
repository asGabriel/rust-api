use chrono::NaiveDate;
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::modules::chat_bot::domain::utils;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewDebtData {
    pub description: String,
    pub amount: Decimal,
    pub account_identification: String,
    pub is_paid: bool,
    pub due_date: Option<NaiveDate>,
}

impl NewDebtData {
    /// Try to create a NewDebtData from parameters.
    /// Supports flexible parameter format:
    /// - Strings = description
    /// - Numbers = amount
    /// - c:N = account_identification (c:1, c:2, etc)
    /// - d:YYYY-MM-DD = due_date
    /// - p:s/n = is_paid (s=true, n=false, empty=false)
    pub fn try_from(parameters: &[String]) -> HttpResult<Self> {
        if parameters.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Comando 'despesa' requer parâmetros: descrição, valor e conta (c:N). Exemplo: despesa natacao 150 c:2",
            )));
        }

        let mut description_parts = Vec::new();
        let mut amount: Option<Decimal> = None;
        let mut account_identification: Option<String> = None;
        let mut due_date: Option<NaiveDate> = None;
        let mut is_paid = false;

        for param in parameters {
            let param = param.trim();

            match param.split_once(':') {
                Some(("c", id)) if !id.is_empty() => {
                    account_identification = Some(id.to_string());
                }
                Some(("c", _)) => {
                    return Err(Box::new(HttpError::bad_request(
                        "Identificação da conta (c:) requer um número. Exemplo: c:1",
                    )));
                }
                Some(("d", date_str)) => {
                    due_date = Some(utils::parse_date(date_str)?);
                }
                Some(("p", flag)) => {
                    is_paid = match flag {
                        "s" => true,
                        "n" => false,
                        "" => false,
                        _ => {
                            return Err(Box::new(HttpError::bad_request(
                                "Flag de pagamento (p:) deve ser 's' (sim) ou 'n' (não)",
                            )))
                        }
                    };
                }
                None => {
                    // Try to parse as number for amount
                    if let Ok(num) = param.parse::<Decimal>() {
                        if num <= Decimal::ZERO {
                            return Err(Box::new(HttpError::bad_request(
                                "Valor deve ser maior que zero",
                            )));
                        }
                        amount = Some(num);
                    } else {
                        // It's a string = part of description
                        description_parts.push(param);
                    }
                }
                Some(_) => {
                    // Unknown prefix, treat as description
                    description_parts.push(param);
                }
            }
        }

        // Validation
        let description = description_parts.join(" ");
        if description.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Descrição não pode estar vazia",
            )));
        }

        let amount = amount.ok_or_else(|| {
            Box::new(HttpError::bad_request(
                "Valor é obrigatório. Use um número para o valor (ex: 150, 150.50)",
            ))
        })?;

        let account_identification = account_identification.ok_or_else(|| {
            Box::new(HttpError::bad_request(
                "Conta é obrigatória. Use o formato c:N (ex: c:1)",
            ))
        })?;

        Ok(NewDebtData {
            description,
            amount,
            account_identification,
            is_paid,
            due_date,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_example1() {
        let params = vec!["natação".to_string(), "150".to_string(), "c:2".to_string()];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "natação");
        assert_eq!(data.amount, Decimal::new(150, 0));
        assert_eq!(data.account_identification, "2");
        assert_eq!(data.is_paid, false);
        assert!(data.due_date.is_none());
    }

    #[test]
    fn test_try_from_example2() {
        let params = vec![
            "mercado".to_string(),
            "400".to_string(),
            "c:1".to_string(),
            "p:s".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "mercado");
        assert_eq!(data.amount, Decimal::new(400, 0));
        assert_eq!(data.account_identification, "1");
        assert_eq!(data.is_paid, true);
    }

    #[test]
    fn test_try_from_with_date_iso() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "c:3".to_string(),
            "d:2025-01-15".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
        assert!(result.unwrap().due_date.is_some());
    }

    #[test]
    fn test_try_from_with_date_brazilian_full() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "c:3".to_string(),
            "d:15/01/2025".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
        assert!(result.unwrap().due_date.is_some());
    }

    #[test]
    fn test_try_from_with_date_brazilian_short() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "c:3".to_string(),
            "d:15/01".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
        assert!(result.unwrap().due_date.is_some());
    }

    #[test]
    fn test_try_from_with_date_hoje() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "c:3".to_string(),
            "d:hoje".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert!(data.due_date.is_some());
        assert_eq!(data.due_date, Some(chrono::Utc::now().date_naive()));
    }

    #[test]
    fn test_try_from_with_date_offset_positive() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "c:3".to_string(),
            "d:+1".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_with_date_offset_negative() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "c:3".to_string(),
            "d:-7".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_empty_params() {
        let params = vec![];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_missing_amount() {
        let params = vec!["mercado".to_string(), "c:1".to_string()];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_missing_account() {
        let params = vec!["mercado".to_string(), "100".to_string()];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_invalid_amount() {
        let params = vec!["mercado".to_string(), "abc".to_string(), "c:1".to_string()];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err()); // Não tem valor numérico válido
    }

    #[test]
    fn test_try_from_invalid_date() {
        let params = vec![
            "mercado".to_string(),
            "100".to_string(),
            "c:1".to_string(),
            "d:invalid".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_paid_false() {
        let params = vec![
            "mercado".to_string(),
            "100".to_string(),
            "c:1".to_string(),
            "p:n".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.is_paid, false);
    }
}
