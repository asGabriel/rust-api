use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};

use crate::modules::finance_manager::{
    domain::payment::Payment, repository::debt::DynDebtRepository,
};

pub type DynPubSubHandler = dyn PubSubHandler + Send + Sync;

#[async_trait]
pub trait PubSubHandler {
    async fn publish_debt_updated_event(&self, payment: &Payment) -> HttpResult<()>;
}

#[derive(Clone)]
pub struct PubSubHandlerImpl {
    pub debt_repository: Arc<DynDebtRepository>,
}

#[async_trait]
impl PubSubHandler for PubSubHandlerImpl {
    async fn publish_debt_updated_event(&self, payment: &Payment) -> HttpResult<()> {
        let mut debt = self
            .debt_repository
            .get_by_id(payment.debt_id())
            .await?
            .or_not_found("debt", &payment.debt_id().to_string())?;

        debt.payment_created(&payment);
        self.debt_repository.update(debt).await?;

        Ok(())
    }
}
