use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, Row};

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

        let row = sqlx::query(
            r#"
                INSERT INTO finance_manager.payment (
                    id,
                    debt_id,
                    account_id,
                    amount,
                    payment_date,
                    created_at,
                    updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING id, debt_id, account_id, amount, payment_date, created_at, updated_at
            "#,
        )
        .bind(payload.id)
        .bind(payload.debt_id)
        .bind(payload.account_id)
        .bind(payload.amount)
        .bind(payload.payment_date)
        .bind(payload.created_at)
        .bind(payload.updated_at)
        .fetch_one(&self.pool)
        .await?;

        let result = PaymentDto {
            id: row.get("id"),
            debt_id: row.get("debt_id"),
            account_id: row.get("account_id"),
            amount: row.get("amount"),
            payment_date: row.get("payment_date"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Ok(Payment::from(result))
    }
}

pub mod dto {
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::payment::Payment;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct PaymentDto {
        pub id: Uuid,
        pub debt_id: Uuid,
        pub account_id: Uuid,
        pub amount: Decimal,
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
                amount: *payment.amount(),
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
                dto.amount,
                dto.payment_date,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
