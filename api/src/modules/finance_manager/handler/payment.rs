use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};

use crate::modules::finance_manager::{
    domain::payment::Payment, repository::payment::DynPaymentRepository,
};

pub type DynPaymentHandler = dyn PaymentHandler + Send + Sync;

#[async_trait]
pub trait PaymentHandler {
    async fn create_payment(&self, payment: Payment) -> HttpResult<Payment>;
}

#[derive(Clone)]
pub struct PaymentHandlerImpl {
    pub pool: Pool<Postgres>,
    pub payment_repository: Arc<DynPaymentRepository>,
}

#[async_trait]
impl PaymentHandler for PaymentHandlerImpl {
    async fn create_payment(&self, payment: Payment) -> HttpResult<Payment> {
        self.payment_repository.insert(&self.pool, payment).await
    }
}
