use chrono::NaiveDate;
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::modules::chat_bot::domain::formatter::ChatFormatterUtils;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewPaymentData {
    pub debt_identification: String,
    pub amount: Option<Decimal>,
    pub discount_amount: Option<Decimal>,
    pub payment_date: Option<NaiveDate>,
}

impl NewPaymentData {
    pub fn try_from(parameters: &[String]) -> HttpResult<Self> {
        if parameters.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Comando 'pagamento' requer pelo menos identificação da dívida. Exemplo: novo-pagamento!123 ou novo-pagamento!123!500",
            )));
        }

        let debt_identification = parameters[0].clone();

        // Amount is optional - parse if provided
        let amount = if parameters.len() >= 2 && !parameters[1].trim().is_empty() {
            Some(parameters[1].parse::<Decimal>().map_err(|_| {
                Box::new(HttpError::bad_request(format!(
                    "Valor inválido: {}",
                    parameters[1]
                )))
            })?)
        } else {
            None
        };

        // Parse payment_date if provided, otherwise use today's date
        let payment_date = if parameters.len() >= 3 && !parameters[2].trim().is_empty() {
            Some(ChatFormatterUtils::parse_friendly_date(&parameters[2])?)
        } else {
            Some(chrono::Utc::now().date_naive())
        };

        Ok(NewPaymentData {
            debt_identification,
            amount,
            discount_amount: None,
            payment_date,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_from_valid_data_with_date() {
        let params = vec![
            "ABCD".to_string(),
            "500".to_string(),
            "2025-01-01".to_string(),
        ];
        let result = NewPaymentData::try_from(&params);
        assert!(result.is_ok());

        let payment_data = result.unwrap();
        assert_eq!(payment_data.debt_identification, "ABCD");
        assert_eq!(
            payment_data.amount,
            Some(rust_decimal::Decimal::new(500, 0))
        );
        assert_eq!(
            payment_data.payment_date,
            Some(chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap())
        );
    }

    #[test]
    fn test_try_from_valid_data_without_date() {
        let params = vec!["ABCD".to_string(), "500".to_string()];
        let result = NewPaymentData::try_from(&params);
        assert!(result.is_ok());

        let payment_data = result.unwrap();
        assert_eq!(payment_data.debt_identification, "ABCD");
        assert_eq!(
            payment_data.amount,
            Some(rust_decimal::Decimal::new(500, 0))
        );
        assert!(payment_data.payment_date.is_some()); // Should use today's date
    }

    #[test]
    fn test_try_from_without_amount() {
        let params = vec!["ABCD".to_string()];
        let result = NewPaymentData::try_from(&params);
        assert!(result.is_ok());

        let payment_data = result.unwrap();
        assert_eq!(payment_data.debt_identification, "ABCD");
        assert_eq!(payment_data.amount, None);
    }

    #[test]
    fn test_try_from_invalid_amount() {
        let params = vec!["ABCD".to_string(), "abc".to_string()];
        let result = NewPaymentData::try_from(&params);
        assert!(result.is_err());
    }
}
