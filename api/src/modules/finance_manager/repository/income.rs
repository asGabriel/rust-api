use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, Row};

use crate::modules::finance_manager::domain::income::Income;

#[async_trait]
pub trait IncomeRepository {
    async fn insert(&self, income: Income) -> HttpResult<Income>;

    // TODO: Add filters
    async fn list(&self) -> HttpResult<Vec<Income>>;
}

pub type DynIncomeRepository = dyn IncomeRepository + Send + Sync;

#[derive(Clone)]
pub struct IncomeRepositoryImpl {
    pool: Pool<Postgres>,
}

impl IncomeRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl IncomeRepository for IncomeRepositoryImpl {
    async fn list(&self) -> HttpResult<Vec<Income>> {
        let rows = sqlx::query(r#"SELECT * FROM finance_manager.income ORDER BY created_at DESC"#)
            .fetch_all(&self.pool)
            .await?;

        let income_entities: Vec<entity::IncomeEntity> = rows
            .into_iter()
            .map(|row| entity::IncomeEntity {
                id: row.get("id"),
                account_id: row.get("account_id"),
                description: row.get("description"),
                amount: row.get("amount"),
                reference: row.get("reference"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .collect();

        Ok(income_entities.into_iter().map(Income::from).collect())
    }

    async fn insert(&self, income: Income) -> HttpResult<Income> {
        let income_entity = entity::IncomeEntity::from(income);

        let row = sqlx::query(
            r#"
            INSERT INTO finance_manager.income (
                id,
                account_id,
                description,
                amount,
                reference,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, account_id, description, amount, reference, created_at, updated_at
            "#,
        )
        .bind(income_entity.id)
        .bind(income_entity.account_id)
        .bind(income_entity.description)
        .bind(income_entity.amount)
        .bind(income_entity.reference)
        .bind(income_entity.created_at)
        .bind(income_entity.updated_at)
        .fetch_one(&self.pool)
        .await?;

        let income_entity = entity::IncomeEntity {
            id: row.get("id"),
            account_id: row.get("account_id"),
            description: row.get("description"),
            amount: row.get("amount"),
            reference: row.get("reference"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Ok(Income::from(income_entity))
    }
}

pub mod entity {
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::income::Income;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct IncomeEntity {
        pub id: Uuid,
        pub account_id: Uuid,
        pub description: String,
        pub amount: Decimal,
        pub reference: NaiveDate,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<Income> for IncomeEntity {
        fn from(income: Income) -> Self {
            IncomeEntity {
                id: *income.id(),
                account_id: *income.account_id(),
                description: income.description().clone(),
                amount: *income.amount(),
                reference: income.reference().clone(),
                created_at: income.created_at().naive_utc(),
                updated_at: income.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<IncomeEntity> for Income {
        fn from(entity: IncomeEntity) -> Self {
            Income::from_row(
                entity.id,
                entity.account_id,
                entity.description,
                entity.amount,
                entity.reference.clone(),
                entity.created_at.and_utc(),
                entity.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
