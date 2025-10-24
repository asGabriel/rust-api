use async_trait::async_trait;
use database::push_filter;
use http_error::{ext::OptionHttpExt, HttpResult};
use sqlx::{Pool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::modules::finance_manager::domain::debt::{Debt, DebtFilters};

#[async_trait]
pub trait DebtRepository {
    async fn list(&self, filters: DebtFilters) -> HttpResult<Vec<Debt>>;

    async fn insert(&self, debt: Debt) -> HttpResult<Debt>;

    async fn get_by_identification(&self, identification: &str) -> HttpResult<Option<Debt>>;

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<Debt>>;

    async fn update(&self, debt: Debt) -> HttpResult<Debt>;
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
    async fn update(&self, debt: Debt) -> HttpResult<Debt> {
        let debt_dto = entity::DebtEntity::from(debt);

        let debt_dto = sqlx::query_as!(
            entity::DebtEntity,
            r#"
            UPDATE finance_manager.debt SET 
                account_id = $1, 
                identification = $2, 
                description = $3, 
                total_amount = $4, 
                paid_amount = $5, 
                discount_amount = $6, 
                remaining_amount = $7, 
                due_date = $8, 
                status = $9, 
                updated_at = $10
            WHERE id = $11 
            RETURNING *
            "#,
            debt_dto.account_id,
            debt_dto.identification,
            debt_dto.description,
            debt_dto.total_amount,
            debt_dto.paid_amount,
            debt_dto.discount_amount,
            debt_dto.remaining_amount,
            debt_dto.due_date,
            debt_dto.status,
            debt_dto.updated_at,
            debt_dto.id,
        )
        .fetch_optional(&self.pool)
        .await?
        .or_not_found("debt", &debt_dto.id.to_string())?;

        Ok(Debt::from(debt_dto))
    }

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<Debt>> {
        let debt: Option<entity::DebtEntity> = sqlx::query_as!(
            entity::DebtEntity,
            r#"SELECT * FROM finance_manager.debt WHERE id = $1"#,
            id,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(debt.map(Debt::from))
    }

    async fn get_by_identification(&self, identification: &str) -> HttpResult<Option<Debt>> {
        let debt: Option<entity::DebtEntity> = sqlx::query_as!(
            entity::DebtEntity,
            r#"SELECT * FROM finance_manager.debt WHERE identification = $1"#,
            identification,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(debt.map(Debt::from))
    }

    async fn insert(&self, debt: Debt) -> HttpResult<Debt> {
        let debt_dto = entity::DebtEntity::from(debt);

        let debt_dto: entity::DebtEntity = sqlx::query_as!(
            entity::DebtEntity,
            r#"
            INSERT INTO finance_manager.debt (
                id, 
                account_id, 
                identification,
                description, 
                total_amount, 
                paid_amount, 
                discount_amount, 
                remaining_amount, 
                due_date,
                status,
                created_at,
                updated_at
            ) 
            VALUES 
                ($1, $2, $3, $4, $5, $6, $7, $8, $9::DATE, $10, $11, $12)
            RETURNING *
        "#,
            debt_dto.id,
            debt_dto.account_id,
            debt_dto.identification,
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

    async fn list(&self, filters: DebtFilters) -> HttpResult<Vec<Debt>> {
        let mut query = QueryBuilder::new("SELECT * FROM finance_manager.debt");
        let mut has_where = false;

        if let Some(ids) = filters.ids() {
            push_filter!(query, &mut has_where, "id IN ($1)", ids);
        }

        if let Some(statuses) = filters.statuses() {
            let status_strings: Vec<String> = statuses.iter().map(|s| s.clone().into()).collect();
            push_filter!(query, &mut has_where, "status IN ($1)", status_strings);
        }

        let query = query.build();
        let rows = query.fetch_all(&self.pool).await?;

        let debt_dtos: Vec<entity::DebtEntity> = rows
            .into_iter()
            .map(|row| entity::DebtEntity {
                id: row.get("id"),
                account_id: row.get("account_id"),
                identification: row.get("identification"),
                description: row.get("description"),
                total_amount: row.get("total_amount"),
                paid_amount: row.get("paid_amount"),
                discount_amount: row.get("discount_amount"),
                remaining_amount: row.get("remaining_amount"),
                due_date: row.get("due_date"),
                status: row.get("status"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

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
        pub identification: String,
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
                identification: debt.identification().clone(),
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
                dto.identification,
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
