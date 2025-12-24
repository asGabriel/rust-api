use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, QueryBuilder};

use crate::modules::finance_manager::{
    domain::debt::installment::{Installment, InstallmentFilters},
    repository::debt::installment::entity::InstallmentEntity,
};

#[async_trait]
pub trait InstallmentRepository {
    async fn insert_many(&self, installments: Vec<Installment>) -> HttpResult<Vec<Installment>>;
    async fn list(&self, filters: &InstallmentFilters) -> HttpResult<Vec<Installment>>;
    async fn update(&self, installment: Installment) -> HttpResult<Installment>;
}

pub type DynInstallmentRepository = dyn InstallmentRepository + Send + Sync;

pub struct InstallmentRepositoryImpl {
    pool: Pool<Postgres>,
}

impl InstallmentRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl InstallmentRepository for InstallmentRepositoryImpl {
    async fn update(&self, installment: Installment) -> HttpResult<Installment> {
        let installment_dto = InstallmentEntity::from(installment);

        let row = sqlx::query(
            r#"
            UPDATE finance_manager.debt_installment SET 
                debt_id = $2,
                installment_id = $3,
                due_date = $4,
                amount = $5,
                is_paid = $6,
                payment_id = $7,
                updated_at = $8
            WHERE installment_id = $1
            RETURNING *
            "#,
        )
        .bind(installment_dto.installment_id)
        .bind(installment_dto.debt_id)
        .bind(installment_dto.installment_id)
        .bind(installment_dto.due_date)
        .bind(installment_dto.amount)
        .bind(installment_dto.is_paid)
        .bind(installment_dto.payment_id)
        .bind(installment_dto.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(Installment::from(InstallmentEntity::from(&row)))
    }

    async fn insert_many(&self, installments: Vec<Installment>) -> HttpResult<Vec<Installment>> {
        let mut tx = self.pool.begin().await?;
        let mut results: Vec<Installment> = Vec::new();

        for installment in installments {
            let payload = InstallmentEntity::from(installment);

            let row = sqlx::query(
                r#"
                    INSERT INTO finance_manager.debt_installment (
                        debt_id,
                        installment_id,
                        due_date,
                        amount,
                        is_paid,
                        payment_id,
                        created_at,
                        updated_at
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    RETURNING *
                "#,
            )
            .bind(payload.debt_id)
            .bind(payload.installment_id)
            .bind(payload.due_date)
            .bind(payload.amount)
            .bind(payload.is_paid)
            .bind(payload.payment_id)
            .bind(payload.created_at)
            .bind(payload.updated_at)
            .fetch_one(&mut *tx)
            .await?;

            results.push(Installment::from(InstallmentEntity::from(&row)));
        }

        tx.commit().await?;

        Ok(results)
    }

    async fn list(&self, filters: &InstallmentFilters) -> HttpResult<Vec<Installment>> {
        let mut builder = QueryBuilder::new("SELECT * FROM finance_manager.debt_installment");
        let has_where = false;

        if let Some(debt_id) = filters.debt_id {
            builder.push(if has_where { " AND " } else { " WHERE " });
            builder.push("debt_id = ");
            builder.push_bind(debt_id);
        }

        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|row| Installment::from(InstallmentEntity::from(&row)))
            .collect())
    }
}

pub mod entity {
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::PgRow;
    use sqlx::Row;
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::debt::installment::Installment;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct InstallmentEntity {
        pub debt_id: Uuid,
        pub installment_id: i32,
        pub due_date: NaiveDate,
        pub amount: Decimal,
        pub is_paid: bool,
        pub payment_id: Option<Uuid>,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<&PgRow> for InstallmentEntity {
        fn from(row: &PgRow) -> Self {
            Self {
                debt_id: row.get("debt_id"),
                installment_id: row.get("installment_id"),
                due_date: row.get("due_date"),
                amount: row.get("amount"),
                is_paid: row.get("is_paid"),
                payment_id: row.get("payment_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        }
    }

    impl From<Installment> for InstallmentEntity {
        fn from(installment: Installment) -> Self {
            Self {
                debt_id: *installment.debt_id(),
                installment_id: *installment.installment_id(),
                due_date: *installment.due_date(),
                amount: *installment.amount(),
                is_paid: *installment.is_paid(),
                payment_id: *installment.payment_id(),
                created_at: installment.created_at().naive_utc(),
                updated_at: installment.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<InstallmentEntity> for Installment {
        fn from(entity: InstallmentEntity) -> Self {
            Installment::from_row(
                entity.debt_id,
                entity.installment_id,
                entity.due_date,
                entity.amount,
                entity.is_paid,
                entity.payment_id,
                entity.created_at.and_utc(),
                entity.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
