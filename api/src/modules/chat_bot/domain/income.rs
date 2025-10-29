use chrono::{NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::modules::chat_bot::domain::utils;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewIncomeData {
    pub description: String,
    pub amount: Decimal,
    pub account_identification: String,
    pub date_reference: NaiveDate,
}

impl NewIncomeData {
    /// Try to create a NewIncomeData from parameters.
    /// Supports flexible parameter format:
    /// - Strings = description
    /// - Numbers = amount
    /// - c:N = account_identification (c:1, c:2, etc)
    /// - d:YYYY-MM-DD or d:DD/MM/YYYY or d:hoje = date_reference
    pub fn try_from(parameters: &[String]) -> HttpResult<Self> {
        if parameters.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Comando 'entrada' requer parâmetros: descrição, valor, conta (c:N) e data (d:). Exemplo: entrada salario 5000 c:1 d:hoje",
            )));
        }

        let mut description_parts = Vec::new();
        let mut amount: Option<Decimal> = None;
        let mut account_identification: Option<String> = None;
        let mut date_reference: Option<NaiveDate> = None;

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
                    date_reference = Some(utils::parse_date(date_str)?);
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

        let date_reference = date_reference.unwrap_or_else(|| Utc::now().date_naive());

        Ok(NewIncomeData {
            description,
            amount,
            account_identification,
            date_reference,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn test_try_from_example1() {
        let params = vec![
            "salario".to_string(),
            "5000".to_string(),
            "c:1".to_string(),
            "d:hoje".to_string(),
        ];
        let result = NewIncomeData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "salario");
        assert_eq!(data.amount, Decimal::new(5000, 0));
        assert_eq!(data.account_identification, "1");
        assert_eq!(data.date_reference, Utc::now().date_naive());
    }

    #[test]
    fn test_try_from_with_date_iso() {
        let params = vec![
            "freelance".to_string(),
            "1500".to_string(),
            "c:2".to_string(),
            "d:2025-01-15".to_string(),
        ];
        let result = NewIncomeData::try_from(&params);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(
            data.date_reference,
            NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()
        );
    }

    #[test]
    fn test_try_from_with_date_brazilian() {
        let params = vec![
            "bonus".to_string(),
            "2000".to_string(),
            "c:1".to_string(),
            "d:15/01/2025".to_string(),
        ];
        let result = NewIncomeData::try_from(&params);
        assert!(result.is_ok());
        assert!(result.unwrap().date_reference.year() == 2025);
    }

    #[test]
    fn test_try_from_without_date_uses_today() {
        let params = vec!["salario".to_string(), "5000".to_string(), "c:1".to_string()];
        let result = NewIncomeData::try_from(&params);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.date_reference, Utc::now().date_naive());
    }

    #[test]
    fn test_try_from_empty_params() {
        let params = vec![];
        let result = NewIncomeData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_missing_amount() {
        let params = vec!["salario".to_string(), "c:1".to_string()];
        let result = NewIncomeData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_missing_account() {
        let params = vec!["salario".to_string(), "5000".to_string()];
        let result = NewIncomeData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_invalid_amount() {
        let params = vec!["salario".to_string(), "abc".to_string(), "c:1".to_string()];
        let result = NewIncomeData::try_from(&params);
        assert!(result.is_err());
    }
}
