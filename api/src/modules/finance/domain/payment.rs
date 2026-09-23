use chrono::{DateTime, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters, DeletedBy};
use uuid::Uuid;

use crate::modules::finance::domain::{debt::Debt, money::MoneyExt};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    id: Uuid,
    client_id: Uuid,
    debt_id: Uuid,
    amount: Decimal,
    payment_date: NaiveDate,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_by: Option<DeletedBy>,
}

impl Payment {
    pub fn new(debt: &Debt, amount: Decimal, payment_date: NaiveDate) -> Self {
        Self {
            id: Uuid::new_v4(),
            client_id: *debt.client_id(),
            debt_id: *debt.id(),
            amount,
            payment_date,
            created_at: Utc::now(),
            updated_at: None,
            deleted_by: None,
        }
    }
}

getters! {
    Payment {
        id: Uuid,
        client_id: Uuid,
        debt_id: Uuid,
        amount: Decimal,
        payment_date: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        deleted_by: Option<DeletedBy>,
    }
}

from_row_constructor! {
    Payment {
        id: Uuid,
        client_id: Uuid,
        debt_id: Uuid,
        amount: Decimal,
        payment_date: NaiveDate,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        deleted_by: Option<DeletedBy>,
    }
}

/// Validates whether an amount can be paid against a given debt.
pub struct PaymentValidator<'a> {
    debt: &'a Debt,
}

impl<'a> PaymentValidator<'a> {
    pub fn new(debt: &'a Debt) -> Self {
        Self { debt }
    }

    pub fn validate(&self, amount: Decimal) -> HttpResult<()> {
        if self.debt.is_installment_parent() {
            return Err(Box::new(HttpError::bad_request(
                "An installment parent debt cannot be paid directly — pay its installments instead",
            )));
        }

        if self.debt.is_settled() {
            return Err(Box::new(HttpError::bad_request("Debt is already settled")));
        }

        if amount <= Decimal::ZERO {
            return Err(Box::new(HttpError::bad_request(
                "Payment amount must be greater than zero",
            )));
        }

        if amount.exceeds_money_scale() {
            return Err(Box::new(HttpError::bad_request(
                "Payment amount must have at most 2 decimal places",
            )));
        }

        if self.debt.exceeds_remaining(amount) {
            return Err(Box::new(HttpError::bad_request(
                "Payment amount cannot exceed the debt's remaining amount",
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PaymentFilters {
    client_id: Option<Uuid>,
    debt_ids: Option<Vec<Uuid>>,
    start_date: Option<NaiveDate>,
    end_date: Option<NaiveDate>,
}

getters!(
    PaymentFilters {
        client_id: Option<Uuid>,
        debt_ids: Option<Vec<Uuid>>,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
    }
);

impl PaymentFilters {
    pub fn new(client_id: Uuid) -> Self {
        Self {
            client_id: Some(client_id),
            ..Default::default()
        }
    }

    pub fn with_optional_debt_ids(mut self, debt_ids: Option<Vec<Uuid>>) -> Self {
        if let Some(ids) = debt_ids {
            self.debt_ids = Some(ids);
        }
        self
    }

    pub fn with_optional_start_date(mut self, start_date: Option<NaiveDate>) -> Self {
        if let Some(d) = start_date {
            self.start_date = Some(d);
        }
        self
    }

    pub fn with_optional_end_date(mut self, end_date: Option<NaiveDate>) -> Self {
        if let Some(d) = end_date {
            self.end_date = Some(d);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::finance::domain::debt::DebtStatus;

    fn d(value: &str) -> Decimal {
        value.parse().unwrap()
    }

    fn due_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 1, 31).unwrap()
    }

    fn debt(total: Decimal) -> Debt {
        Debt::new(
            Uuid::nil(),
            "debt".to_string(),
            total,
            due_date(),
            None,
            None,
            None,
            None,
        )
    }

    fn installment_plan(total: Decimal, count: i32) -> (Debt, Vec<Debt>) {
        let mut parent = Debt::new(
            Uuid::nil(),
            "plan".to_string(),
            total,
            due_date(),
            None,
            None,
            None,
            Some(count),
        );
        let children = parent.generate_installment_children(31).unwrap();
        (parent, children)
    }

    #[test]
    fn partial_payment_keeps_debt_open() {
        let mut debt = debt(d("100"));

        debt.apply_payment(d("40")).unwrap();

        assert_eq!(*debt.paid_amount(), d("40"));
        assert_eq!(*debt.remaining_amount(), d("60"));
        assert_eq!(*debt.status(), DebtStatus::Open);
    }

    #[test]
    fn paying_the_remaining_settles_the_debt() {
        let mut debt = debt(d("100"));

        debt.apply_payment(d("40")).unwrap();
        debt.apply_payment(d("60")).unwrap();

        assert_eq!(*debt.remaining_amount(), Decimal::ZERO);
        assert_eq!(*debt.status(), DebtStatus::Settled);
    }

    #[test]
    fn payment_above_remaining_is_rejected() {
        let mut debt = debt(d("100"));
        debt.apply_payment(d("40")).unwrap();

        assert!(debt.apply_payment(d("60.01")).is_err());
        assert_eq!(*debt.paid_amount(), d("40"));
    }

    #[test]
    fn non_positive_payment_is_rejected() {
        let mut debt = debt(d("100"));

        assert!(debt.apply_payment(Decimal::ZERO).is_err());
        assert!(debt.apply_payment(d("-1")).is_err());
    }

    #[test]
    fn payment_with_more_than_two_decimal_places_is_rejected() {
        let mut debt = debt(d("100"));

        assert!(debt.apply_payment(d("33.335")).is_err());
        assert_eq!(*debt.paid_amount(), Decimal::ZERO);
    }

    #[test]
    fn settled_debt_cannot_be_paid() {
        let mut debt = debt(d("100"));
        debt.apply_payment(d("100")).unwrap();

        assert!(debt.apply_payment(d("1")).is_err());
    }

    #[test]
    fn installment_parent_cannot_be_paid() {
        let (mut parent, _) = installment_plan(d("300"), 3);

        assert!(parent.apply_payment(d("100")).is_err());
    }

    #[test]
    fn installment_child_can_be_paid_out_of_order() {
        let (_, mut children) = installment_plan(d("300"), 3);

        children[2].apply_payment(d("100")).unwrap();

        assert_eq!(*children[2].status(), DebtStatus::Settled);
    }

    #[test]
    fn refund_reopens_a_settled_debt() {
        let mut debt = debt(d("100"));
        debt.apply_payment(d("100")).unwrap();

        debt.revert_payment(d("30")).unwrap();

        assert_eq!(*debt.paid_amount(), d("70"));
        assert_eq!(*debt.remaining_amount(), d("30"));
        assert_eq!(*debt.status(), DebtStatus::Open);
    }

    #[test]
    fn refund_above_paid_is_rejected() {
        let mut debt = debt(d("100"));
        debt.apply_payment(d("10")).unwrap();

        assert!(debt.revert_payment(d("10.01")).is_err());
    }

    #[test]
    fn parent_syncs_balance_and_status_from_children() {
        let (mut parent, mut children) = installment_plan(d("300"), 3);

        children[0].apply_payment(d("100")).unwrap();
        children[1].apply_payment(d("50")).unwrap();
        parent.sync_with_children(&children);

        assert_eq!(*parent.paid_amount(), d("150"));
        assert_eq!(*parent.remaining_amount(), d("150"));
        assert_eq!(*parent.status(), DebtStatus::Open);

        children[1].apply_payment(d("50")).unwrap();
        children[2].apply_payment(d("100")).unwrap();
        parent.sync_with_children(&children);

        assert_eq!(*parent.remaining_amount(), Decimal::ZERO);
        assert_eq!(*parent.status(), DebtStatus::Settled);
    }
}
