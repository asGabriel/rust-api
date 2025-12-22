use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;

use crate::modules::finance_manager::{
    domain::{debt::Debt, payment::Payment},
    repository::debt::DynDebtRepository,
};

pub type DynPubSubHandler = dyn PubSubHandler + Send + Sync;

#[async_trait]
pub trait PubSubHandler {
    async fn process_debt_payment(
        &self,
        debt: Debt,
        payment: &Payment,
        force_settlement: bool,
    ) -> HttpResult<Debt>;
}

#[derive(Clone)]
pub struct PubSubHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
}

#[async_trait]
impl PubSubHandler for PubSubHandlerImpl {
    async fn process_debt_payment(
        &self,
        mut debt: Debt,
        payment: &Payment,
        force_settlement: bool,
    ) -> HttpResult<Debt> {
        if force_settlement {
            println!("force_settlement");
            debt.force_settlement(&payment);
        } else {
            println!("process_payment");
            debt.process_payment(&payment);
        }

        dbg!(&debt, "process_debt_payment");

        self.debt_repository.update(debt.clone()).await?;

        Ok(debt)
    }
}
