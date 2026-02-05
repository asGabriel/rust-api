use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};

use crate::modules::finance_manager::{
    domain::{debt::Debt, financial_instrument::FinancialInstrument, payment::Payment},
    handler::{
        payment::use_cases::{CreatePaymentRequest, PaymentBasicData},
        pubsub::DynPubSubHandler,
    },
    repository::{
        debt::DynDebtRepository,
        financial_instrument::DynFinancialInstrumentRepository,
        payment::{use_cases::PaymentFilters, DynPaymentRepository},
    },
};

pub type DynPaymentHandler = dyn PaymentHandler + Send + Sync;

use uuid::Uuid;

#[async_trait]
pub trait PaymentHandler {
    async fn create_payment(&self, request: CreatePaymentRequest) -> HttpResult<Payment>;
    async fn list_payments(
        &self,
        client_id: Uuid,
        filters: PaymentFilters,
    ) -> HttpResult<Vec<Payment>>;
    async fn refund_payment(&self, client_id: Uuid, payment_id: Uuid) -> HttpResult<()>;
}

#[derive(Clone)]
pub struct PaymentHandlerImpl {
    pub payment_repository: Arc<DynPaymentRepository>,
    pub debt_repository: Arc<DynDebtRepository>,
    pub financial_instrument_repository: Arc<DynFinancialInstrumentRepository>,
    pub pubsub: Arc<DynPubSubHandler>,
}

#[async_trait]
impl PaymentHandler for PaymentHandlerImpl {
    async fn create_payment(&self, request: CreatePaymentRequest) -> HttpResult<Payment> {
        let (debt, instrument, payment_data, reconcile) =
            self.extract_payment_data_from_request(request).await?;
        let payment = Payment::new(&debt, instrument.id(), &payment_data);

        // Validate BEFORE inserting (skip validation when reconcile is true)
        if !reconcile {
            self.pubsub.validate_payment(&debt, &payment).await?;
        }

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

    async fn list_payments(
        &self,
        client_id: Uuid,
        filters: PaymentFilters,
    ) -> HttpResult<Vec<Payment>> {
        let filters = filters.with_client_id(client_id);
        self.payment_repository.list(&filters).await
    }

    async fn refund_payment(&self, client_id: Uuid, payment_id: Uuid) -> HttpResult<()> {
        let payment = self
            .payment_repository
            .get_by_id(&payment_id)
            .await?
            .or_not_found("payment", payment_id.to_string())?;

        if payment.client_id() != &client_id {
            return Err(Box::new(http_error::HttpError::forbidden(
                "You don't have permission to refund this payment",
            )));
        }

        let debt = self
            .debt_repository
            .get_by_id(payment.debt_id())
            .await?
            .or_not_found("debt", payment.debt_id().to_string())?;

        self.pubsub.reverse_payment(debt, &payment).await?;

        self.payment_repository.delete(&payment_id).await?;

        Ok(())
    }
}

impl PaymentHandlerImpl {
    async fn extract_payment_data_from_request(
        &self,
        request: CreatePaymentRequest,
    ) -> HttpResult<(Debt, FinancialInstrument, PaymentBasicData, bool)> {
        let (debt, instrument, payment_data, reconcile) = match request {
            CreatePaymentRequest::PaymentRequestFromIdentification(data) => (
                self.debt_repository
                    .get_by_identification(&data.debt_identification)
                    .await?
                    .or_not_found("debt", &data.debt_identification)?,
                self.financial_instrument_repository
                    .get_by_identification(&data.financial_instrument_identification)
                    .await?
                    .or_not_found(
                        "financial_instrument",
                        &data.financial_instrument_identification,
                    )?,
                data.payment_basic_data,
                data.reconcile,
            ),
            CreatePaymentRequest::PaymentRequestFromUuid(data) => (
                self.debt_repository
                    .get_by_id(&data.debt_id)
                    .await?
                    .or_not_found("debt", data.debt_id.to_string())?,
                self.financial_instrument_repository
                    .get_by_id(data.financial_instrument_id)
                    .await?
                    .or_not_found(
                        "financial_instrument",
                        data.financial_instrument_id.to_string(),
                    )?,
                data.payment_basic_data,
                data.reconcile,
            ),
        };

        Ok((debt, instrument, payment_data, reconcile))
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
        pub financial_instrument_identification: String,
        #[serde(default)]
        pub reconcile: bool,
        #[serde(flatten)]
        pub payment_basic_data: PaymentBasicData,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct PaymentRequestFromUuid {
        pub debt_id: Uuid,
        pub financial_instrument_id: Uuid,
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
