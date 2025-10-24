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
        let parameters = vec![
            "mercado da semana".to_string(),
            "500".to_string(),
            "ABCD".to_string(),
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "mercado da semana");
        assert_eq!(data.amount, Decimal::new(500, 0));
        assert_eq!(data.account_identification, "ABCD");
    }

    #[test]
    fn test_try_from_decimal_amount() {
        let parameters = vec![
            "compra no shopping".to_string(),
            "150.50".to_string(),
            "EFGH".to_string(),
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "compra no shopping");
        assert_eq!(data.amount, Decimal::new(15050, 2));
        assert_eq!(data.account_identification, "EFGH");
    }

    #[test]
    fn test_try_from_trims_description() {
        let parameters = vec![
            "  farmácia  ".to_string(),
            "25.99".to_string(),
            "IJKL".to_string(),
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "farmácia");
        assert_eq!(data.amount, Decimal::new(2599, 2));
        assert_eq!(data.account_identification, "IJKL");
    }

    #[test]
    fn test_try_from_insufficient_parameters() {
        let test_cases = vec![
            vec![],                                             // empty
            vec!["description".to_string()],                    // only description
            vec!["description".to_string(), "100".to_string()], // missing account_id
        ];

        for parameters in test_cases {
            let result = NewDebtData::try_from(&parameters);
            assert!(
                result.is_err(),
                "Expected error for parameters: {:?}",
                parameters
            );
        }
    }

    #[test]
    fn test_try_from_empty_description() {
        let parameters = vec!["".to_string(), "100".to_string(), "MNOP".to_string()];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_whitespace_description() {
        let parameters = vec!["   ".to_string(), "100".to_string(), "QRST".to_string()];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_invalid_amount() {
        let parameters = vec![
            "description".to_string(),
            "abc".to_string(),
            "UVWX".to_string(),
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_zero_amount() {
        let parameters = vec![
            "description".to_string(),
            "0".to_string(),
            "YZAB".to_string(),
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_negative_amount() {
        let parameters = vec![
            "description".to_string(),
            "-100".to_string(),
            "CDEF".to_string(),
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_extra_parameters() {
        let parameters = vec![
            "description".to_string(),
            "100".to_string(),
            "GHIJ".to_string(),
            "extra".to_string(),
            "param".to_string(),
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.description, "description");
        assert_eq!(data.amount, Decimal::new(100, 0));
        assert_eq!(data.account_identification, "GHIJ");
    }

    #[test]
    fn test_try_from_account_id_too_short() {
        let parameters = vec![
            "description".to_string(),
            "100".to_string(),
            "ABC".to_string(), // only 3 characters
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("exatamente 4 caracteres"));
    }

    #[test]
    fn test_try_from_account_id_too_long() {
        let parameters = vec![
            "description".to_string(),
            "100".to_string(),
            "ABCDE".to_string(), // 5 characters
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("exatamente 4 caracteres"));
    }

    #[test]
    fn test_try_from_account_id_with_spaces() {
        let parameters = vec![
            "description".to_string(),
            "100".to_string(),
            " AB ".to_string(), // spaces around, but only 2 chars when trimmed
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_err());

        let error = result.unwrap_err();
        assert!(error.message.contains("exatamente 4 caracteres"));
    }

    #[test]
    fn test_try_from_account_id_valid_with_spaces() {
        let parameters = vec![
            "description".to_string(),
            "100".to_string(),
            " ABCD ".to_string(), // spaces around, but 4 chars when trimmed
        ];

        let result = NewDebtData::try_from(&parameters);
        assert!(result.is_ok());

        let data = result.unwrap();
        assert_eq!(data.account_identification, "ABCD");
    }
}
