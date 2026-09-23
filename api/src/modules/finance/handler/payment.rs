use async_trait::async_trait;
use chrono::Utc;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use uuid::Uuid;

use util::DeletedBy;

use crate::modules::finance::{
    domain::payment::{Payment, PaymentFilters},
    handler::payment::use_cases::CreatePaymentRequest,
    repository::{debt::DynDebtRepository, payment::DynPaymentRepository},
};
use std::sync::Arc;

pub type DynPaymentHandler = dyn PaymentHandler + Send + Sync;

#[async_trait]
pub trait PaymentHandler {
    async fn create_payment(
        &self,
        client_id: Uuid,
        request: CreatePaymentRequest,
    ) -> HttpResult<Payment>;

    async fn list_payments(
        &self,
        client_id: Uuid,
        filters: &PaymentFilters,
    ) -> HttpResult<Vec<Payment>>;

    async fn refund_payment(
        &self,
        client_id: Uuid,
        user_id: Uuid,
        payment_id: Uuid,
    ) -> HttpResult<()>;
}

#[derive(Clone)]
pub struct PaymentHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
    pub payment_repository: Arc<DynPaymentRepository>,
}

#[async_trait]
impl PaymentHandler for PaymentHandlerImpl {
    async fn create_payment(
        &self,
        client_id: Uuid,
        request: CreatePaymentRequest,
    ) -> HttpResult<Payment> {
        let mut debt = self
            .debt_repository
            .get_by_id(&request.debt_id)
            .await?
            .or_not_found("debt", request.debt_id.to_string())?;

        if debt.client_id() != &client_id {
            return Err(Box::new(HttpError::forbidden(
                "You don't have permission to pay this debt",
            )));
        }

        let amount = request.amount.unwrap_or(*debt.remaining_amount());
        debt.apply_payment(amount)?;

        let payment_date = request
            .payment_date
            .unwrap_or_else(|| Utc::now().date_naive());
        let payment = Payment::new(&debt, amount, payment_date);

        self.payment_repository.register(payment, debt).await
    }

    async fn list_payments(
        &self,
        client_id: Uuid,
        filters: &PaymentFilters,
    ) -> HttpResult<Vec<Payment>> {
        let built = PaymentFilters::new(client_id)
            .with_optional_debt_ids(filters.debt_ids().clone())
            .with_optional_start_date(*filters.start_date())
            .with_optional_end_date(*filters.end_date());

        self.payment_repository.list(&built).await
    }

    async fn refund_payment(
        &self,
        client_id: Uuid,
        user_id: Uuid,
        payment_id: Uuid,
    ) -> HttpResult<()> {
        let payment = self
            .payment_repository
            .get_by_id(&payment_id)
            .await?
            .or_not_found("payment", payment_id.to_string())?;

        if payment.client_id() != &client_id {
            return Err(Box::new(HttpError::forbidden(
                "You don't have permission to refund this payment",
            )));
        }

        let mut debt = self
            .debt_repository
            .get_by_id(payment.debt_id())
            .await?
            .or_not_found("debt", payment.debt_id().to_string())?;

        debt.revert_payment(*payment.amount())?;

        self.payment_repository
            .refund(payment, debt, DeletedBy::new(user_id))
            .await
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreatePaymentRequest {
        pub debt_id: Uuid,
        /// Omitted = pays the debt's whole remaining amount.
        pub amount: Option<Decimal>,
        /// Omitted = today.
        pub payment_date: Option<NaiveDate>,
    }
}
