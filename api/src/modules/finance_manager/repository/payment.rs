use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, QueryBuilder, Row};

use crate::modules::finance_manager::{
    domain::payment::Payment,
    repository::payment::{dto::PaymentDto, use_cases::PaymentFilters},
};

pub type DynPaymentRepository = dyn PaymentRepository + Send + Sync;

#[async_trait]
pub trait PaymentRepository {
    async fn insert(&self, payment: Payment) -> HttpResult<Payment>;
    async fn list(&self, filters: &PaymentFilters) -> HttpResult<Vec<Payment>>;
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
                    client_id,
                    debt_id,
                    account_id,
                    amount,
                    payment_date,
                    created_at,
                    updated_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                RETURNING *
            "#,
        )
        .bind(payload.id)
        .bind(payload.client_id)
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
            client_id: row.get("client_id"),
            debt_id: row.get("debt_id"),
            account_id: row.get("account_id"),
            amount: row.get("amount"),
            payment_date: row.get("payment_date"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Ok(Payment::from(result))
    }

    async fn list(&self, filters: &PaymentFilters) -> HttpResult<Vec<Payment>> {
        let mut builder: QueryBuilder<Postgres> = QueryBuilder::new(
            r#"
                SELECT * FROM finance_manager.payment WHERE 1=1
            "#,
        );

        if let Some(client_id) = &filters.client_id {
            builder.push(" AND client_id = ");
            builder.push_bind(client_id);
        }

        if let Some(debt_ids) = &filters.debt_ids {
            builder.push(" AND debt_id = ANY(");
            builder.push_bind(debt_ids);
            builder.push(")");
        }

        if let Some(account_ids) = &filters.account_ids {
            builder.push(" AND account_id = ANY(");
            builder.push_bind(account_ids);
            builder.push(")");
        }

        if let Some(start_date) = &filters.start_date {
            builder.push(" AND payment_date >= ");
            builder.push_bind(start_date);
        }

        if let Some(end_date) = &filters.end_date {
            builder.push(" AND payment_date <= ");
            builder.push_bind(end_date);
        }

        builder.push(" ORDER BY payment_date DESC");

        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        let payments: Vec<Payment> = rows
            .iter()
            .map(|row| {
                Payment::from(PaymentDto {
                    id: row.get("id"),
                    client_id: row.get("client_id"),
                    debt_id: row.get("debt_id"),
                    account_id: row.get("account_id"),
                    amount: row.get("amount"),
                    payment_date: row.get("payment_date"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            })
            .collect();

        Ok(payments)
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
        pub client_id: Uuid,
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
                client_id: *payment.client_id(),
                debt_id: *payment.debt_id(),
                account_id: *payment.account_id(),
                amount: *payment.amount(),
                payment_date: *payment.payment_date(),
                created_at: payment.created_at().naive_utc(),
                updated_at: payment.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<PaymentDto> for Payment {
        fn from(dto: PaymentDto) -> Self {
            Payment::from_row(
                dto.id,
                dto.client_id,
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

pub mod use_cases {
    use chrono::NaiveDate;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize, Deserialize, Default)]
    #[serde(rename_all = "camelCase")]
    pub struct PaymentFilters {
        pub client_id: Option<Uuid>,
        pub debt_ids: Option<Vec<Uuid>>,
        pub account_ids: Option<Vec<Uuid>>,
        pub start_date: Option<NaiveDate>,
        pub end_date: Option<NaiveDate>,
    }

    impl PaymentFilters {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn with_client_id(mut self, client_id: Uuid) -> Self {
            self.client_id = Some(client_id);
            self
        }

        pub fn with_debt_ids(mut self, debt_ids: Vec<Uuid>) -> Self {
            self.debt_ids = Some(debt_ids);
            self
        }

        pub fn with_account_ids(mut self, account_ids: Vec<Uuid>) -> Self {
            self.account_ids = Some(account_ids);
            self
        }

        pub fn with_start_date(mut self, start_date: NaiveDate) -> Self {
            self.start_date = Some(start_date);
            self
        }

        pub fn with_end_date(mut self, end_date: NaiveDate) -> Self {
            self.end_date = Some(end_date);
            self
        }
    }
}
