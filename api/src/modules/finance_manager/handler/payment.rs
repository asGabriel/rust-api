use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};

use crate::modules::finance_manager::{
    domain::payment::Payment,
    handler::{payment::use_cases::CreatePaymentRequest, pubsub::DynPubSubHandler},
    repository::{debt::DynDebtRepository, payment::DynPaymentRepository},
};

pub type DynPaymentHandler = dyn PaymentHandler + Send + Sync;

#[async_trait]
pub trait PaymentHandler {
    async fn create_payment(&self, request: CreatePaymentRequest) -> HttpResult<Payment>;
}

#[derive(Clone)]
pub struct PaymentHandlerImpl {
    pub payment_repository: Arc<DynPaymentRepository>,
    pub debt_repository: Arc<DynDebtRepository>,
    pub pubsub: Arc<DynPubSubHandler>,
}

#[async_trait]
impl PaymentHandler for PaymentHandlerImpl {
    async fn create_payment(&self, request: CreatePaymentRequest) -> HttpResult<Payment> {
        let (mut debt, payment_data) = match request {
            CreatePaymentRequest::PaymentRequestFromIdentification(data) => (
                self.debt_repository
                    .get_by_identification(&data.debt_identification)
                    .await?
                    .or_not_found("debt", &data.debt_identification)?,
                data.payment_basic_data,
            ),
            CreatePaymentRequest::PaymentRequestFromUuid(data) => (
                self.debt_repository
                    .get_by_id(&data.debt_id)
                    .await?
                    .or_not_found("debt", &data.debt_id.to_string())?,
                data.payment_basic_data,
            ),
        };

        let payment = Payment::new(&debt, &payment_data);
        debt.process_payment(&payment, false)?;

        let payment = self.payment_repository.insert(payment).await?;
        self.debt_repository.update(debt).await?;

        Ok(payment)
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::debt::Debt;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum CreatePaymentRequest {
        PaymentRequestFromIdentification(PaymentRequestFromIdentification),
        PaymentRequestFromUuid(PaymentRequestFromUuid),
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PaymentRequestFromIdentification {
        pub debt_identification: String,
        #[serde(flatten)]
        pub payment_basic_data: PaymentBasicData,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PaymentRequestFromUuid {
        pub debt_id: Uuid,
        #[serde(flatten)]
        pub payment_basic_data: PaymentBasicData,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PaymentBasicData {
        pub payment_date: NaiveDate,
        pub amount: Option<Decimal>,
    }

    impl PaymentBasicData {
        pub fn amount(&self, debt: &Debt) -> Decimal {
            self.amount.unwrap_or(*debt.remaining_amount())
        }
    }
}
