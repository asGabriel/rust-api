use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};

use crate::modules::finance_manager::domain::payment::Payment;

pub type DynPaymentRepository = dyn PaymentRepository + Send + Sync;

#[async_trait]
pub trait PaymentRepository {
    async fn insert(&self, pool: &Pool<Postgres>, payment: Payment) -> HttpResult<Payment>;
}

pub struct PaymentRepositoryImpl;

#[async_trait]
impl PaymentRepository for PaymentRepositoryImpl {
    async fn insert(&self, _pool: &Pool<Postgres>, payment: Payment) -> HttpResult<Payment> {
        // TODO: Implement actual database insertion using pool
        // Example:
        // sqlx::query!("INSERT INTO payments (...) VALUES (...)")
        //     .execute(pool)
        //     .await?;
        Ok(payment)
    }
}
