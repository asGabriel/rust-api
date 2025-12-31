use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;

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
}

#[derive(Clone)]
pub struct PubSubHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
    pub installment_repository: Arc<DynInstallmentRepository>,
}

impl PubSubHandlerImpl {
    // TODO: understand if this is needed
    async fn process_installments(&self, debt: &Debt, payment: &Payment) -> HttpResult<()> {
        if !debt.has_installments() {
            return Ok(());
        }

        let installments = self
            .installment_repository
            .list(&InstallmentFilters::new().with_debt_ids(&[*debt.id()]))
            .await?;
        if let Some(latest_unpaid) = Installment::get_latest_unpaid(&installments) {
            let mut latest_unpaid = latest_unpaid.clone();

            latest_unpaid.process_payment(payment)?;
            self.installment_repository.update(latest_unpaid).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl PubSubHandler for PubSubHandlerImpl {
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
        debt.process_payment(&payment)?;

        self.debt_repository.update(debt.clone()).await?;

        Ok(debt)
    }
}
