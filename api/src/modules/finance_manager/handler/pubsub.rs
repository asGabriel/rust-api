use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};

use crate::modules::finance_manager::{
    domain::{
        debt::{
            installment::{Installment, InstallmentFilters},
            Debt,
        },
        payment::Payment,
    },
    repository::debt::{installment::DynInstallmentRepository, DynDebtRepository},
};

pub type DynPubSubHandler = dyn PubSubHandler + Send + Sync;

#[async_trait]
pub trait PubSubHandler {
    /// Validates payment before inserting into database.
    async fn validate_payment(&self, debt: &Debt, payment: &Payment) -> HttpResult<()>;

    /// Processes the debt payment and updates the debt data.
    async fn process_debt_payment(&self, debt: Debt, payment: &Payment) -> HttpResult<Debt>;

    /// Reconciles the debt with the actual payment amount when they differ.
    /// Necessary when the payment is executed and the debt data not matches the payment data
    /// and the debt must be updated.
    async fn reconcile_debt_with_actual_payment(
        &self,
        debt: Debt,
        payment: &Payment,
    ) -> HttpResult<Debt>;

    /// Reverses a payment, updating the debt and installment (if applicable).
    async fn reverse_payment(&self, debt: Debt, payment: &Payment) -> HttpResult<Debt>;
}

#[derive(Clone)]
pub struct PubSubHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
    pub installment_repository: Arc<DynInstallmentRepository>,
}

impl PubSubHandlerImpl {
    async fn process_latest_installment_for_debt(&self, payment: &Payment) -> HttpResult<()> {
        let installments = self
            .installment_repository
            .list(&InstallmentFilters::new().with_debt_ids(&[*payment.debt_id()]))
            .await?;

        let mut latest_installment = Installment::get_latest_unpaid(&installments)
            .or_not_found("latest installment for debt", payment.debt_id().to_string())?
            .clone();

        latest_installment.process_payment(payment)?;

        self.installment_repository
            .update(latest_installment.clone())
            .await?;

        Ok(())
    }
}

#[async_trait]
impl PubSubHandler for PubSubHandlerImpl {
    async fn validate_payment(&self, debt: &Debt, payment: &Payment) -> HttpResult<()> {
        if debt.has_installments() {
            let installments = self
                .installment_repository
                .list(&InstallmentFilters::new().with_debt_ids(&[*debt.id()]))
                .await?;

            let latest_installment = Installment::get_latest_unpaid(&installments)
                .or_not_found("unpaid installment", debt.id().to_string())?;

            latest_installment.validate_payment(payment)?;
        } else {
            debt.validate_payment_amount(payment)?;
        }

        Ok(())
    }

    async fn reconcile_debt_with_actual_payment(
        &self,
        mut debt: Debt,
        payment: &Payment,
    ) -> HttpResult<Debt> {
        debt.reconcile_with_actual_payment(payment)?;

        self.debt_repository.update(debt.clone()).await?;

        Ok(debt)
    }

    async fn process_debt_payment(&self, mut debt: Debt, payment: &Payment) -> HttpResult<Debt> {
        if debt.has_installments() {
            self.process_latest_installment_for_debt(payment).await?;
        }
        debt.process_payment(payment)?;

        self.debt_repository.update(debt.clone()).await?;
        Ok(debt)
    }

    async fn reverse_payment(&self, mut debt: Debt, payment: &Payment) -> HttpResult<Debt> {
        if debt.has_installments() {
            let installments = self
                .installment_repository
                .list(&InstallmentFilters::new().with_payment_id(*payment.id()))
                .await?;

            if let Some(installment) = installments.first() {
                let mut installment = installment.clone();
                installment.reverse_payment()?;
                self.installment_repository.update(installment).await?;
            }
        }

        debt.reverse_payment(payment)?;
        self.debt_repository.update(debt.clone()).await?;

        Ok(debt)
    }
}
