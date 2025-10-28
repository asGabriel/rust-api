use chrono::{Datelike, Duration, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};

/// Parse a date string in various formats
/// Supports:
/// - Keywords: "hoje", "amanhã", "ontem"
/// - Offsets: "+1", "-7" (days from today)
/// - Brazilian format: "15/01/2025" or "15/01"
/// - ISO format: "2025-01-15"
pub fn parse_date(date_str: &str) -> HttpResult<NaiveDate> {
    let today = Utc::now().date_naive();

    // Try parsing as special keywords first
    match date_str.to_lowercase().as_str() {
        "hoje" => return Ok(today),
        "amanhã" | "amanha" => return Ok(today + Duration::days(1)),
        "ontem" => return Ok(today - Duration::days(1)),
        _ => {}
    }

    // Try parsing as offset (e.g., +1, -7, +30)
    if let Some(offset_str) = date_str.strip_prefix('+') {
        if let Ok(days) = offset_str.parse::<i64>() {
            return Ok(today + Duration::days(days));
        }
    } else if let Some(offset_str) = date_str.strip_prefix('-') {
        if let Ok(days) = offset_str.parse::<i64>() {
            return Ok(today - Duration::days(days));
        }
    }

    // Try parsing as Brazilian format DD/MM/YYYY or DD/MM
    if date_str.contains('/') {
        let parts: Vec<&str> = date_str.split('/').collect();

        match parts.len() {
            2 => {
                // DD/MM - assume current year
                let day = parts[0].parse::<u32>().map_err(|_| {
                    Box::new(HttpError::bad_request(format!(
                        "Dia inválido: '{}'",
                        parts[0]
                    )))
                })?;
                let month = parts[1].parse::<u32>().map_err(|_| {
                    Box::new(HttpError::bad_request(format!(
                        "Mês inválido: '{}'",
                        parts[1]
                    )))
                })?;

                return NaiveDate::from_ymd_opt(today.year(), month, day).ok_or_else(|| {
                    Box::new(HttpError::bad_request(format!(
                        "Data inválida: '{}/{}'. Exemplo: 15/01",
                        day, month
                    )))
                });
            }
            3 => {
                // DD/MM/YYYY
                let day = parts[0].parse::<u32>().map_err(|_| {
                    Box::new(HttpError::bad_request(format!(
                        "Dia inválido: '{}'",
                        parts[0]
                    )))
                })?;
                let month = parts[1].parse::<u32>().map_err(|_| {
                    Box::new(HttpError::bad_request(format!(
                        "Mês inválido: '{}'",
                        parts[1]
                    )))
                })?;
                let year = parts[2].parse::<i32>().map_err(|_| {
                    Box::new(HttpError::bad_request(format!(
                        "Ano inválido: '{}'",
                        parts[2]
                    )))
                })?;

                return NaiveDate::from_ymd_opt(year, month, day).ok_or_else(|| {
                    Box::new(HttpError::bad_request(format!(
                        "Data inválida: '{}/{}/{}'. Exemplo: 15/01/2025",
                        day, month, year
                    )))
                });
            }
            _ => {}
        }
    }

    // Try parsing as ISO format (YYYY-MM-DD)
    if date_str.contains('-') {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            return Ok(date);
        }
    }

    // If all parsing attempts failed
    Err(Box::new(HttpError::bad_request(format!(
        "Data inválida: '{}'. Use um destes formatos:\n\
        • Formato brasileiro: 15/01/2025 ou 15/01\n\
        • Formato ISO: 2025-01-15\n\
        • Palavras: hoje, amanhã, ontem\n\
        • Offsets: +1 (amanhã), -7 (há 7 dias)",
        date_str
    ))))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_hoje() {
        let result = parse_date("hoje");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Utc::now().date_naive());
    }

    #[test]
    fn test_parse_date_amanha() {
        let result = parse_date("amanhã");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_date_ontem() {
        let result = parse_date("ontem");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_date_offset_positive() {
        let result = parse_date("+1");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_date_offset_negative() {
        let result = parse_date("-7");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_date_brazilian_full() {
        let result = parse_date("15/01/2025");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()
        );
    }

    #[test]
    fn test_parse_date_brazilian_short() {
        let result = parse_date("15/01");
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_date_iso() {
        let result = parse_date("2025-01-15");
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()
        );
    }

    #[test]
    fn test_parse_date_invalid() {
        let result = parse_date("invalid");
        assert!(result.is_err());
    }
}
