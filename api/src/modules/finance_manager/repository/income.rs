use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, QueryBuilder, Row};

use crate::modules::finance_manager::{
    domain::income::Income, repository::income::use_cases::IncomeListFilters,
};

#[async_trait]
pub trait IncomeRepository {
    async fn insert(&self, income: Income) -> HttpResult<Income>;

    async fn list(&self, filters: &IncomeListFilters) -> HttpResult<Vec<Income>>;
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
    async fn list(&self, filters: &IncomeListFilters) -> HttpResult<Vec<Income>> {
        let mut builder = QueryBuilder::new(format!(
            "SELECT * FROM finance_manager.income WHERE client_id = {}",
            filters.client_id()
        ));

        if let Some(start_date) = filters.start_date() {
            builder.push(" AND reference >= ");
            builder.push_bind(start_date);
        }

        if let Some(end_date) = filters.end_date() {
            builder.push(" AND reference <= ");
            builder.push_bind(end_date);
        }

        if let Some(financial_instrument_ids) = filters.financial_instrument_ids() {
            builder.push(" AND financial_instrument_id = ANY(");
            builder.push_bind(financial_instrument_ids);
            builder.push(")");
        }

        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                Income::from(entity::IncomeEntity {
                    id: row.get("id"),
                    client_id: row.get("client_id"),
                    financial_instrument_id: row.get("financial_instrument_id"),
                    description: row.get("description"),
                    amount: row.get("amount"),
                    reference: row.get("reference"),
                    created_at: row.get("created_at"),
                    updated_at: row.get("updated_at"),
                })
            })
            .collect())
    }

    async fn insert(&self, income: Income) -> HttpResult<Income> {
        let income_entity = entity::IncomeEntity::from(income);

        let row = sqlx::query(
            r#"
            INSERT INTO finance_manager.income (
                id,
                client_id,
                financial_instrument_id,
                description,
                amount,
                reference,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(income_entity.id)
        .bind(income_entity.client_id)
        .bind(income_entity.financial_instrument_id)
        .bind(income_entity.description)
        .bind(income_entity.amount)
        .bind(income_entity.reference)
        .bind(income_entity.created_at)
        .bind(income_entity.updated_at)
        .fetch_one(&self.pool)
        .await?;

        let income_entity = entity::IncomeEntity {
            id: row.get("id"),
            client_id: row.get("client_id"),
            financial_instrument_id: row.get("financial_instrument_id"),
            description: row.get("description"),
            amount: row.get("amount"),
            reference: row.get("reference"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Ok(Income::from(income_entity))
    }
}

pub mod use_cases {
    use chrono::NaiveDate;
    use serde::{Deserialize, Serialize};
    use util::getters;
    use uuid::Uuid;

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct IncomeListFilters {
        client_id: Uuid,
        start_date: Option<NaiveDate>,
        end_date: Option<NaiveDate>,
        financial_instrument_ids: Option<Vec<Uuid>>,
    }

    impl IncomeListFilters {
        pub fn new(client_id: Uuid) -> Self {
            Self {
                client_id,
                ..Default::default()
            }
        }

        pub fn with_start_date(mut self, start_date: Option<NaiveDate>) -> Self {
            self.start_date = start_date;
            self
        }

        pub fn with_end_date(mut self, end_date: Option<NaiveDate>) -> Self {
            self.end_date = end_date;
            self
        }

        pub fn with_financial_instrument_ids(
            mut self,
            financial_instrument_ids: Option<Vec<Uuid>>,
        ) -> Self {
            self.financial_instrument_ids = financial_instrument_ids;
            self
        }
    }

    getters!(
        IncomeListFilters {
            client_id: Uuid,
            start_date: Option<NaiveDate>,
            end_date: Option<NaiveDate>,
            financial_instrument_ids: Option<Vec<Uuid>>,
        }
    );
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
        pub client_id: Uuid,
        pub financial_instrument_id: Uuid,
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
                client_id: *income.client_id(),
                financial_instrument_id: *income.financial_instrument_id(),
                description: income.description().clone(),
                amount: *income.amount(),
                reference: *income.reference(),
                created_at: income.created_at().naive_utc(),
                updated_at: income.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<IncomeEntity> for Income {
        fn from(entity: IncomeEntity) -> Self {
            Income::from_row(
                entity.id,
                entity.client_id,
                entity.financial_instrument_id,
                entity.description,
                entity.amount,
                entity.reference,
                entity.created_at.and_utc(),
                entity.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
