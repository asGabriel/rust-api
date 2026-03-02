use chrono::{DateTime, NaiveDate, Utc};
use http_error::{HttpError, HttpResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

use crate::modules::finance_manager::domain::payment::Payment;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Installment {
    debt_id: Uuid,
    installment_id: i32,
    due_date: NaiveDate,
    amount: Decimal,
    is_paid: bool,
    payment_id: Option<Uuid>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Installment {
    pub fn new(debt_id: Uuid, installment_id: i32, due_date: NaiveDate, amount: Decimal) -> Self {
        Self {
            debt_id,
            installment_id,
            due_date,
            amount,
            is_paid: false,
            payment_id: None,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn process_payment(&mut self, payment: &Payment) -> HttpResult<()> {
        self.validate_payment(payment)?;

        self.set_as_paid(*payment.id());
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    pub fn validate_payment(&self, payment: &Payment) -> HttpResult<()> {
        if *self.is_paid() {
            return Err(Box::new(HttpError::bad_request("Installment already paid")));
        }

        if *payment.amount() != *self.amount() {
            return Err(Box::new(HttpError::bad_request(
                "Payment amount differs from installment amount",
            )));
        }

        Ok(())
    }

    pub fn set_as_paid(&mut self, payment_id: Uuid) {
        self.is_paid = true;
        self.payment_id = Some(payment_id);
        self.updated_at = Some(Utc::now());
    }

    pub fn reverse_payment(&mut self) -> HttpResult<()> {
        if !self.is_paid {
            return Err(Box::new(HttpError::bad_request(
                "Cannot reverse: installment is not paid",
            )));
        }

        self.is_paid = false;
        self.payment_id = None;
        self.updated_at = Some(Utc::now());

        Ok(())
    }

    pub fn get_latest_unpaid(installments: &[Self]) -> Option<&Self> {
        installments
            .iter()
            .filter(|i| !i.is_paid())
            .min_by_key(|i| i.installment_id())
    }
}

getters!(
    Installment {
        debt_id: Uuid,
        installment_id: i32,
        due_date: NaiveDate,
        amount: Decimal,
        is_paid: bool,
        payment_id: Option<Uuid>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
);

from_row_constructor! {
    Installment {
        debt_id: Uuid,
        installment_id: i32,
        due_date: NaiveDate,
        amount: Decimal,
        is_paid: bool,
        payment_id: Option<Uuid>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
