use chrono::NaiveDate;
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::modules::chat_bot::domain::utils;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewDebtData {
    pub description: String,
    pub amount: Decimal,
    pub due_date: NaiveDate,
    pub category_name: String,
    pub account_identification: Option<String>,
    pub installment_number: Option<i32>,
}

impl NewDebtData {
    pub fn is_paid(&self) -> bool {
        self.account_identification.is_some()
    }

    /// Try to create a NewDebtData from parameters.
    /// Supports flexible parameter format:
    /// - Strings = description
    /// - Numbers = amount
    /// - c:N = account_identification (c:1, c:2, etc) - quando presente, indica que a despesa está paga
    /// - d:YYYY-MM-DD = due_date (obrigatório)
    /// - cat:Nome = category_name (obrigatório)
    pub fn try_from(parameters: &[String]) -> HttpResult<Self> {
        if parameters.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Comando 'despesa' requer parâmetros: descrição, valor, data (d:YYYY-MM-DD) e categoria (cat:Nome). Exemplo: despesa natacao 150 d:2025-01-15 cat:Esportes",
            )));
        }

        let mut description_parts = Vec::new();
        let mut amount: Option<Decimal> = None;
        let mut due_date: Option<NaiveDate> = None;
        let mut category_name: Option<String> = None;
        let mut account_identification: Option<String> = None;
        let mut installment_number: Option<i32> = None;

        for param in parameters {
            let param = param.trim();

            match param.split_once(':') {
                Some(("c", id)) => {
                    if id.is_empty() {
                        return Err(Box::new(HttpError::bad_request(
                            "Identificação da conta (c:) não pode estar vazia. Exemplo: c:1",
                        )));
                    }
                    account_identification = Some(id.to_string());
                }
                Some(("d", date_str)) => {
                    due_date = Some(utils::parse_date(date_str)?);
                }
                Some(("cat", name)) => {
                    if name.is_empty() {
                        return Err(Box::new(HttpError::bad_request(
                            "Nome da categoria (cat:) não pode estar vazio. Exemplo: cat:Alimentação",
                        )));
                    }
                    category_name = Some(name.to_uppercase());
                }
                Some(("i", number)) => {
                    let num = number.parse::<i32>().map_err(|_| {
                        HttpError::bad_request(format!(
                            "Número de parcelas (i:) deve ser um número inteiro válido. Exemplo: i:3"
                        ))
                    })?;
                    if num <= 0 {
                        return Err(Box::new(HttpError::bad_request(
                            "Número de parcelas (i:) deve ser maior que zero. Exemplo: i:3",
                        )));
                    }
                    installment_number = Some(num);
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

        let category_name = category_name.ok_or_else(|| {
            Box::new(HttpError::bad_request(
                "Categoria é obrigatória. Use o formato cat:Nome (ex: cat:Alimentação)",
            ))
        })?;

        let due_date = due_date.ok_or_else(|| {
            Box::new(HttpError::bad_request(
                "Data de vencimento é obrigatória. Use o formato d:YYYY-MM-DD, d:DD/MM/YYYY, d:DD/MM, d:hoje ou d:+N/-N (ex: d:2025-01-15, d:15/01/2025, d:hoje, d:+1)",
            ))
        })?;

        Ok(NewDebtData {
            description,
            amount,
            due_date,
            category_name,
            account_identification,
            installment_number,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_example1() {
        let params = vec![
            "natação".to_string(),
            "150".to_string(),
            "d:2025-01-15".to_string(),
            "cat:Esportes".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "natação");
        assert_eq!(data.amount, Decimal::new(150, 0));
        assert_eq!(data.is_paid(), false);
        assert_eq!(data.category_name, "ESPORTES");
        assert_eq!(data.account_identification, None);
    }

    #[test]
    fn test_try_from_example2() {
        let params = vec![
            "mercado".to_string(),
            "400".to_string(),
            "d:2025-01-20".to_string(),
            "c:1".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "mercado");
        assert_eq!(data.amount, Decimal::new(400, 0));
        assert_eq!(data.is_paid(), true);
        assert_eq!(data.category_name, "ALIMENTAÇÃO");
        assert_eq!(data.account_identification, Some("1".to_string()));
    }

    #[test]
    fn test_try_from_with_date_iso() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "d:2025-01-15".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_with_date_brazilian_full() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "d:15/01/2025".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_with_date_brazilian_short() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "d:15/01".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_with_date_hoje() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "d:hoje".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
        let data = result.unwrap();
        assert_eq!(data.due_date, chrono::Utc::now().date_naive());
    }

    #[test]
    fn test_try_from_with_date_offset_positive() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "d:+1".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_try_from_with_date_offset_negative() {
        let params = vec![
            "almoço".to_string(),
            "30".to_string(),
            "d:-7".to_string(),
            "cat:Alimentação".to_string(),
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
        let params = vec![
            "mercado".to_string(),
            "d:2025-01-15".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_missing_due_date() {
        let params = vec![
            "mercado".to_string(),
            "100".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Data de vencimento é obrigatória"));
    }

    #[test]
    fn test_try_from_invalid_amount() {
        let params = vec![
            "mercado".to_string(),
            "abc".to_string(),
            "d:2025-01-15".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err()); // Não tem valor numérico válido
    }

    #[test]
    fn test_try_from_invalid_date() {
        let params = vec![
            "mercado".to_string(),
            "100".to_string(),
            "d:invalid".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_unpaid() {
        let params = vec![
            "mercado".to_string(),
            "100".to_string(),
            "d:2025-01-15".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.is_paid(), false);
        assert_eq!(data.account_identification, None);
    }

    #[test]
    fn test_try_from_missing_category() {
        let params = vec![
            "mercado".to_string(),
            "100".to_string(),
            "d:2025-01-15".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("Categoria é obrigatória"));
    }

    #[test]
    fn test_try_from_with_category_accent() {
        let params = vec![
            "Psicóloga".to_string(),
            "300".to_string(),
            "c:9".to_string(),
            "cat:saúde".to_string(),
            "d:10/11".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "Psicóloga");
        assert_eq!(data.amount, Decimal::new(300, 0));
        assert_eq!(data.category_name, "SAÚDE"); // Deve ser SAÚDE, não Psicóloga!
        assert_eq!(data.is_paid(), true);
        assert_eq!(data.account_identification, Some("9".to_string()));
    }

    #[test]
    fn test_try_from_paid_with_account() {
        let params = vec![
            "mercado".to_string(),
            "100".to_string(),
            "d:2025-01-15".to_string(),
            "c:1".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.is_paid(), true);
        assert_eq!(data.account_identification, Some("1".to_string()));
    }

    #[test]
    fn test_try_from_unpaid_without_account() {
        let params = vec![
            "mercado".to_string(),
            "100".to_string(),
            "d:2025-01-15".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.is_paid(), false);
        assert_eq!(data.account_identification, None);
    }

    #[test]
    fn test_try_from_unpaid_with_account_ignored() {
        // Se c: está presente, sempre será considerado pago, mesmo que não faça sentido
        let params = vec![
            "mercado".to_string(),
            "100".to_string(),
            "d:2025-01-15".to_string(),
            "c:2".to_string(),
            "cat:Alimentação".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        // Se tem conta, é considerado pago
        assert_eq!(data.is_paid(), true);
        assert_eq!(data.account_identification, Some("2".to_string()));
    }
}
