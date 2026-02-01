use chrono::{Datelike, NaiveDate};

/// Creates a date with the specified day, adjusting to the last valid day
/// if the month doesn't have enough days.
///
/// # Arguments
/// * `year` - The year
/// * `month` - The month (1-12)
/// * `day` - The desired day of month
///
/// # Returns
/// A NaiveDate with the specified day, or the last day of the month if the
/// specified day exceeds the number of days in that month.
///
/// # Example
/// ```
/// use util::date::date_with_day_or_last;
///
/// // Normal case
/// let date = date_with_day_or_last(2026, 1, 15);
/// assert_eq!(date.day(), 15);
///
/// // February with day 31 -> uses 28
/// let date = date_with_day_or_last(2026, 2, 31);
/// assert_eq!(date.day(), 28);
///
/// // Leap year February with day 31 -> uses 29
/// let date = date_with_day_or_last(2024, 2, 31);
/// assert_eq!(date.day(), 29);
/// ```
pub fn date_with_day_or_last(year: i32, month: u32, day: u32) -> NaiveDate {
    let max_day = last_day_of_month(year, month);
    let actual_day = day.min(max_day);
    NaiveDate::from_ymd_opt(year, month, actual_day).unwrap()
}

/// Returns the last day of the specified month.
///
/// # Arguments
/// * `year` - The year
/// * `month` - The month (1-12)
///
/// # Returns
/// The last day of the month (28, 29, 30, or 31)
pub fn last_day_of_month(year: i32, month: u32) -> u32 {
    // Get first day of next month, then go back one day
    let next_month_first = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    };

    next_month_first.pred_opt().unwrap().day()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_date_with_day_or_last_normal() {
        let date = date_with_day_or_last(2026, 1, 15);
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 1, 15).unwrap());
    }

    #[test]
    fn test_date_with_day_or_last_february() {
        let date = date_with_day_or_last(2026, 2, 31);
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    }

    #[test]
    fn test_date_with_day_or_last_leap_year() {
        let date = date_with_day_or_last(2024, 2, 31);
        assert_eq!(date, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
    }

    #[test]
    fn test_date_with_day_or_last_april() {
        let date = date_with_day_or_last(2026, 4, 31);
        assert_eq!(date, NaiveDate::from_ymd_opt(2026, 4, 30).unwrap());
    }

    #[test]
    fn test_last_day_of_month() {
        assert_eq!(last_day_of_month(2026, 1), 31);
        assert_eq!(last_day_of_month(2026, 2), 28);
        assert_eq!(last_day_of_month(2024, 2), 29); // leap year
        assert_eq!(last_day_of_month(2026, 4), 30);
        assert_eq!(last_day_of_month(2026, 12), 31);
    }
}
