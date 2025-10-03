use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};

use crate::modules::finance_manager::domain::debt::{Debt, DebtFilters};

#[async_trait]
pub trait DebtRepository {
    async fn list(&self, filters: DebtFilters) -> HttpResult<Vec<Debt>>;

    async fn insert(&self, debt: Debt) -> HttpResult<Debt>;
}

pub type DynDebtRepository = dyn DebtRepository + Send + Sync;

#[derive(Clone)]
pub struct DebtRepositoryImpl {
    pool: Pool<Postgres>,
}

impl DebtRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl DebtRepository for DebtRepositoryImpl {
    async fn insert(&self, debt: Debt) -> HttpResult<Debt> {
        let debt_dto = entity::DebtEntity::from(debt);

        let debt_dto: entity::DebtEntity = sqlx::query_as!(
            entity::DebtEntity,
            r#"
            INSERT INTO finance_manager.debt (
                id, 
                account_id, 
                description, 
                total_amount, 
                paid_amount, 
                discount_amount, 
                remaining_amount, 
                due_date, status, created_at, updated_at) 
            VALUES 
                ($1, $2, $3, $4, $5, $6, $7, $8::DATE, $9, $10, $11)
            RETURNING *
        "#,
            debt_dto.id,
            debt_dto.account_id,
            debt_dto.description,
            debt_dto.total_amount,
            debt_dto.paid_amount,
            debt_dto.discount_amount,
            debt_dto.remaining_amount,
            debt_dto.due_date,
            debt_dto.status,
            debt_dto.created_at,
            debt_dto.updated_at
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Debt::from(debt_dto))
    }

    async fn list(&self, _filters: DebtFilters) -> HttpResult<Vec<Debt>> {
        let debt_dtos: Vec<entity::DebtEntity> = sqlx::query_as!(entity::DebtEntity, "SELECT * FROM finance_manager.debt")
            .fetch_all(&self.pool)
            .await?;

        let debts = debt_dtos.into_iter().map(Debt::from).collect();
        Ok(debts)
    }
}

pub mod entity {
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use sqlx::prelude::FromRow;
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::debt::Debt;

    #[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
    pub struct DebtEntity {
        pub id: Uuid,
        pub account_id: Uuid,
        pub description: String,
        pub total_amount: Decimal,
        pub paid_amount: Decimal,
        pub discount_amount: Decimal,
        pub remaining_amount: Decimal,
        pub due_date: NaiveDate,
        pub status: String,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<Debt> for DebtEntity {
        fn from(debt: Debt) -> Self {
            DebtEntity {
                id: *debt.id(),
                account_id: *debt.account_id(),
                description: debt.description().clone(),
                total_amount: *debt.total_amount(),
                paid_amount: *debt.paid_amount(),
                discount_amount: *debt.discount_amount(),
                remaining_amount: *debt.remaining_amount(),
                due_date: *debt.due_date(),
                status: debt.status().clone().into(),
                created_at: debt.created_at().naive_utc(),
                updated_at: debt.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<DebtEntity> for Debt {
        fn from(dto: DebtEntity) -> Self {
            Debt::from_row(
                dto.id,
                dto.account_id,
                dto.description,
                dto.total_amount,
                dto.paid_amount,
                dto.discount_amount,
                dto.remaining_amount,
                dto.due_date,
                dto.status.into(),
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
