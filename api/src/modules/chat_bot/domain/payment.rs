use chrono::NaiveDate;
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::modules::chat_bot::domain::utils;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewPaymentData {
    pub debt_identification: String,
    pub amount: Option<Decimal>,
    pub discount_amount: Option<Decimal>,
    pub payment_date: Option<NaiveDate>,
    pub settled: bool,
}

impl NewPaymentData {
    pub fn try_from(parameters: &[String]) -> HttpResult<Self> {
        if parameters.is_empty() {
            return Err(Box::new(HttpError::bad_request(
                "Comando 'pagamento' requer pelo menos identificação da dívida. Exemplo: novo-pagamento!123 ou novo-pagamento!123!500",
            )));
        }

        let mut debt_identification: String = String::new();
        let mut amount: Option<Decimal> = None;
        let mut payment_date: Option<NaiveDate> = None;
        let mut settled = false;

        for param in parameters {
            let param = param.trim();

            match param.split_once(':') {
                Some(("id", id)) if !id.is_empty() => {
                    debt_identification = id.to_string();
                }
                Some(("id", _)) => {
                    return Err(Box::new(HttpError::bad_request(
                        "Identificação da dívida (id:) é obrigatória",
                    )));
                }
                Some(("d", date_str)) => {
                    payment_date = Some(utils::parse_date(date_str)?);
                }
                Some(("baixa", flag)) => {
                    settled = match flag {
                        "s" => true,
                        "n" => false,
                        "" => false,
                        _ => false,
                    };
                }
                None => {
                    if let Ok(num) = param.parse::<Decimal>() {
                        if num <= Decimal::ZERO {
                            return Err(Box::new(HttpError::bad_request(
                                "Valor deve ser maior que zero",
                            )));
                        }
                        amount = Some(num);
                    }
                }
                _ => {
                    if debt_identification.is_empty() {
                        return Err(Box::new(HttpError::bad_request("Comando inválido")));
                    }
                }
            }
        }

        Ok(NewPaymentData {
            debt_identification,
            amount,
            discount_amount: None,
            payment_date,
            settled,
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
