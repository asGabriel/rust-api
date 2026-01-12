use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};

use crate::modules::finance_manager::{
    domain::{account::BankAccount, debt::Debt, payment::Payment},
    handler::{
        payment::use_cases::{CreatePaymentRequest, PaymentBasicData},
        pubsub::DynPubSubHandler,
    },
    repository::{
        account::DynAccountRepository, debt::DynDebtRepository, payment::DynPaymentRepository,
    },
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
    pub account_repository: Arc<DynAccountRepository>,
    pub pubsub: Arc<DynPubSubHandler>,
}

#[async_trait]
impl PaymentHandler for PaymentHandlerImpl {
    async fn create_payment(&self, request: CreatePaymentRequest) -> HttpResult<Payment> {
        let (debt, account, payment_data, reconcile) =
            self.extract_payment_data_from_request(request).await?;
        let payment = Payment::new(&debt, account.id(), &payment_data);

        let payment = self.payment_repository.insert(payment).await?;

        if reconcile {
            self.pubsub
                .reconcile_debt_with_actual_payment(debt, &payment)
                .await?;
        } else {
            self.pubsub.process_debt_payment(debt, &payment).await?;
        }

        Ok(payment)
    }
}

impl PaymentHandlerImpl {
    async fn extract_payment_data_from_request(
        &self,
        request: CreatePaymentRequest,
    ) -> HttpResult<(Debt, BankAccount, PaymentBasicData, bool)> {
        let (debt, account, payment_data, reconcile) = match request {
            CreatePaymentRequest::PaymentRequestFromIdentification(data) => (
                self.debt_repository
                    .get_by_identification(&data.debt_identification)
                    .await?
                    .or_not_found("debt", &data.debt_identification)?,
                self.account_repository
                    .get_by_identification(&data.account_identification)
                    .await?
                    .or_not_found("account", &data.account_identification)?,
                data.payment_basic_data,
                data.reconcile,
            ),
            CreatePaymentRequest::PaymentRequestFromUuid(data) => (
                self.debt_repository
                    .get_by_id(&data.debt_id)
                    .await?
                    .or_not_found("debt", data.debt_id.to_string())?,
                self.account_repository
                    .get_by_id(data.account_id)
                    .await?
                    .or_not_found("account", data.account_id.to_string())?,
                data.payment_basic_data,
                data.reconcile,
            ),
        };

        Ok((debt, account, payment_data, reconcile))
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
        pub account_identification: String,
        #[serde(default)]
        pub reconcile: bool,
        #[serde(flatten)]
        pub payment_basic_data: PaymentBasicData,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PaymentRequestFromUuid {
        pub debt_id: Uuid,
        pub account_id: Uuid,
        #[serde(default)]
        pub reconcile: bool,
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
            self.amount.unwrap_or_else(|| debt.installment_amount())
        }
    }
}
