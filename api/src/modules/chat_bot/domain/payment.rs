use chrono::NaiveDate;
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewPaymentData {
    pub debt_identification: String,
    pub amount: Decimal,
    pub discount_amount: Option<Decimal>,
    pub payment_date: Option<NaiveDate>,
}

impl NewPaymentData {
    pub fn try_from(parameters: &[String]) -> HttpResult<Self> {
        if parameters.len() < 2 {
            return Err(Box::new(HttpError::bad_request(
                "Comando 'pagamento' requer pelo menos 2 parâmetros: identificação da dívida e valor. Exemplo: /pagamento!ABCD!500 ou /pagamento!ABCD!500!2025-01-01",
            )));
        }

        let debt_identification = parameters[0].clone();
        let amount = parameters[1].parse::<Decimal>().map_err(|_| {
            Box::new(HttpError::bad_request(format!(
                "Valor inválido: {}",
                parameters[1]
            )))
        })?;

        // Parse payment_date if provided, otherwise use today's date
        let payment_date = if parameters.len() >= 3 && !parameters[2].trim().is_empty() {
            parameters[2].parse::<NaiveDate>().map_err(|_| {
                Box::new(HttpError::bad_request(format!(
                    "Data de pagamento inválida: {}",
                    parameters[2]
                )))
            })?
        } else {
            chrono::Utc::now().date_naive()
        };

        Ok(NewPaymentData {
            debt_identification,
            amount,
            discount_amount: None,
            payment_date: Some(payment_date),
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
        assert_eq!(payment_data.amount, rust_decimal::Decimal::new(500, 0));
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
        assert_eq!(payment_data.amount, rust_decimal::Decimal::new(500, 0));
        assert!(payment_data.payment_date.is_some()); // Should use today's date
    }

    #[test]
    fn test_try_from_invalid_amount() {
        let params = vec!["ABCD".to_string(), "abc".to_string()];
        let result = NewPaymentData::try_from(&params);
        assert!(result.is_err());
    }

    #[test]
    fn test_try_from_insufficient_params() {
        let params = vec!["ABCD".to_string()];
        let result = NewPaymentData::try_from(&params);
        assert!(result.is_err());
    }
}
