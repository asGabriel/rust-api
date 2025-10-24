use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewDebtData {
    pub description: String,
    pub amount: Decimal,
    pub account_identification: String,
}

impl NewDebtData {
    /// Try to create a NewDebtData from parameters.
    /// Returns HttpResult with validation errors if validation fails.
    pub fn try_from(parameters: &[String]) -> HttpResult<Self> {
        if parameters.len() < 3 {
            return Err(Box::new(HttpError::bad_request(
                "Comando 'novo' requer 3 parâmetros: descrição, valor e identificação da conta (4 caracteres). Exemplo: /novo!descricao!valor!ABCD",
            )));
        }

        let description = parameters[0].clone();
        let amount_str = &parameters[1];
        let account_id = &parameters[2];

        if description.trim().is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Descrição não pode estar vazia",
            )));
        }

        // Parse amount
        let amount = amount_str.parse::<Decimal>().map_err(|_| {
            Box::new(HttpError::bad_request(format!(
                "Valor inválido: '{}'. Use um número válido (ex: 100, 150.50)",
                amount_str
            )))
        })?;

        // Validate amount is positive
        if amount <= Decimal::ZERO {
            return Err(Box::new(HttpError::bad_request(
                "Valor deve ser maior que zero",
            )));
        }

        // Validate account identification
        let account_id_trimmed = account_id.trim();
        if account_id_trimmed.len() != 4 {
            return Err(Box::new(HttpError::bad_request(format!(
                "Identificação da conta deve ter exatamente 4 caracteres. Recebido: '{}' ({} caracteres)",
                account_id_trimmed,
                account_id_trimmed.len()
            ))));
        }

        Ok(NewDebtData {
            description: description.trim().to_string(),
            amount,
            account_identification: account_id_trimmed.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_valid_data() {
        let params = vec![
            "mercado da semana".to_string(),
            "500".to_string(),
            "ABCD".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "mercado da semana");
        assert_eq!(data.amount, Decimal::new(500, 0));
        assert_eq!(data.account_identification, "ABCD");
    }

    #[test]
    fn test_try_from_insufficient_params() {
        let params = vec!["descrição".to_string(), "100".to_string()];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_empty_description() {
        let params = vec!["".to_string(), "100".to_string(), "ABCD".to_string()];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_invalid_amount() {
        let params = vec![
            "descrição".to_string(),
            "abc".to_string(),
            "ABCD".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_invalid_account_id_length() {
        let params = vec![
            "descrição".to_string(),
            "100".to_string(),
            "ABC".to_string(),
        ];
        let result = NewDebtData::try_from(&params);
        assert!(result.is_err());
    }
}
