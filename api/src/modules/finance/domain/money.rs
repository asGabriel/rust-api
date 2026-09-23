use rust_decimal::Decimal;

/// Decimal places stored by every monetary column (`DECIMAL(10, 2)`).
pub const MONEY_SCALE: u32 = 2;

pub trait MoneyExt {
    /// `true` when the value has more decimal places than the database
    /// stores. Trailing zeros are ignored (`10.500` is fine, `10.005` is not).
    /// Such values are rejected instead of rounded, since Postgres rounds each
    /// column independently and would break `remaining = total - paid`.
    fn exceeds_money_scale(&self) -> bool;
}

impl MoneyExt for Decimal {
    fn exceeds_money_scale(&self) -> bool {
        self.normalize().scale() > MONEY_SCALE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(value: &str) -> Decimal {
        value.parse().unwrap()
    }

    #[test]
    fn accepts_up_to_two_decimal_places() {
        assert!(!d("10").exceeds_money_scale());
        assert!(!d("10.5").exceeds_money_scale());
        assert!(!d("10.55").exceeds_money_scale());
    }

    #[test]
    fn ignores_trailing_zeros() {
        assert!(!d("10.500").exceeds_money_scale());
    }

    #[test]
    fn rejects_more_than_two_decimal_places() {
        assert!(d("10.005").exceeds_money_scale());
        assert!(d("33.335").exceeds_money_scale());
    }
}
