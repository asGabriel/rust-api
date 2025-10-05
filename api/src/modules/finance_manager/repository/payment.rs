use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};

use crate::modules::finance_manager::{
    domain::payment::Payment, repository::payment::dto::PaymentDto,
};

pub type DynPaymentRepository = dyn PaymentRepository + Send + Sync;

#[async_trait]
pub trait PaymentRepository {
    async fn insert(&self, payment: Payment) -> HttpResult<Payment>;
}

#[derive(Clone)]
pub struct PaymentRepositoryImpl {
    pool: Pool<Postgres>,
}

impl PaymentRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl PaymentRepository for PaymentRepositoryImpl {
    async fn insert(&self, payment: Payment) -> HttpResult<Payment> {
        let payload = PaymentDto::from(payment);

        let result = sqlx::query_as!(
            PaymentDto,
            r#"
                INSERT INTO finance_manager.payment (id, debt_id, account_id, total_amount, principal_amount, discount_amount, payment_date, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7::DATE, $8, $9)
                RETURNING id, debt_id, account_id, total_amount, principal_amount, discount_amount, payment_date, created_at, updated_at
            "#,
            payload.id,
            payload.debt_id,
            payload.account_id,
            payload.total_amount,
            payload.principal_amount,
            payload.discount_amount,
            payload.payment_date,
            payload.created_at,
            payload.updated_at,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Payment::from(result))
    }
}

pub mod dto {
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use sqlx::FromRow;
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::payment::Payment;

    #[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
    pub struct PaymentDto {
        pub id: Uuid,
        pub debt_id: Uuid,
        pub account_id: Uuid,
        pub total_amount: Decimal,
        pub discount_amount: Decimal,
        pub principal_amount: Decimal,
        pub payment_date: NaiveDate,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<Payment> for PaymentDto {
        fn from(payment: Payment) -> Self {
            PaymentDto {
                id: *payment.id(),
                debt_id: *payment.debt_id(),
                account_id: *payment.account_id(),
                total_amount: *payment.total_amount(),
                discount_amount: *payment.discount_amount(),
                principal_amount: *payment.principal_amount(),
                payment_date: payment.payment_date().clone(),
                created_at: payment.created_at().naive_utc(),
                updated_at: payment.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<PaymentDto> for Payment {
        fn from(dto: PaymentDto) -> Self {
            Payment::from_row(
                dto.id,
                dto.debt_id,
                dto.account_id,
                dto.total_amount,
                dto.principal_amount,
                dto.discount_amount,
                dto.payment_date,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
