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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstallmentFilters {
    pub debt_ids: Option<Vec<Uuid>>,
    pub is_paid: Option<bool>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
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

    fn validate_payment(&self, payment: &Payment) -> HttpResult<()> {
        if *self.is_paid() {
            return Err(Box::new(HttpError::bad_request("Parcela já paga")));
        }

        if *payment.amount() != *self.amount() {
            return Err(Box::new(HttpError::bad_request(
                "Valor do pagamento diferente do valor da parcela",
            )));
        }

        Ok(())
    }

    pub fn set_as_paid(&mut self, payment_id: Uuid) {
        self.is_paid = true;
        self.payment_id = Some(payment_id);
        self.updated_at = Some(Utc::now());
    }

    pub fn get_latest_unpaid(installments: &[Self]) -> Option<&Self> {
        installments
            .iter()
            .filter(|i| !i.is_paid())
            .min_by_key(|i| i.installment_id())
    }
}

impl InstallmentFilters {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub fn with_debt_ids(mut self, debt_ids: &[Uuid]) -> Self {
        self.debt_ids = Some(debt_ids.to_vec());
        self
    }

    pub fn with_is_paid(mut self, is_paid: bool) -> Self {
        self.is_paid = Some(is_paid);
        self
    }

    pub fn with_start_date(mut self, start_date: NaiveDate) -> Self {
        self.start_date = Some(start_date);
        self
    }

    pub fn with_end_date(mut self, end_date: NaiveDate) -> Self {
        self.end_date = Some(end_date);
        self
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
