use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, QueryBuilder, Row};

use crate::modules::finance_manager::domain::debt::recurrence::{Recurrence, RecurrenceFilters};

use entity::RecurrenceEntity;

pub type DynRecurrenceRepository = dyn RecurrenceRepository + Send + Sync;

#[async_trait]
pub trait RecurrenceRepository {
    async fn insert(&self, recurrence: Recurrence) -> HttpResult<Recurrence>;

    // TODO: Add filters
    async fn list(&self, filters: &RecurrenceFilters) -> HttpResult<Vec<Recurrence>>;
}

#[derive(Clone)]
pub struct RecurrenceRepositoryImpl {
    pool: Pool<Postgres>,
}

impl RecurrenceRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl RecurrenceRepository for RecurrenceRepositoryImpl {
    async fn list(&self, filters: &RecurrenceFilters) -> HttpResult<Vec<Recurrence>> {
        let mut builder = QueryBuilder::new("SELECT * FROM finance_manager.recurrence");
        let mut has_where = false;
        if let Some(next_run_date) = filters.next_run_date() {
            builder.push(if has_where { " AND " } else { " WHERE " });
            builder.push("next_run_date = ");
            builder.push_bind(next_run_date);
            has_where = true;
        }
        if let Some(active) = filters.active() {
            builder.push(if has_where { " AND " } else { " WHERE " });
            builder.push("active = ");
            builder.push_bind(active);
            has_where = true;
        }
        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        let results: Vec<RecurrenceEntity> = rows
            .into_iter()
            .map(|r| RecurrenceEntity {
                id: r.get("id"),
                account_id: r.get("account_id"),
                description: r.get("description"),
                amount: r.get("amount"),
                active: r.get("active"),
                start_date: r.get("start_date"),
                end_date: r.get("end_date"),
                day_of_month: r.get("day_of_month"),
                next_run_date: r.get("next_run_date"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect();

        Ok(results.into_iter().map(Recurrence::from).collect())
    }

    async fn insert(&self, recurrence: Recurrence) -> HttpResult<Recurrence> {
        let payload = RecurrenceEntity::from(recurrence);

        let row = sqlx::query(
            r#"
            INSERT INTO finance_manager.recurrence (id, account_id, description, amount, active, start_date, end_date, day_of_month, next_run_date, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, account_id, description, amount, active, start_date, end_date, day_of_month, next_run_date, created_at, updated_at
        "#
        )
        .bind(payload.id)
        .bind(payload.account_id)
        .bind(payload.description)
        .bind(payload.amount)
        .bind(payload.active)
        .bind(payload.start_date)
        .bind(payload.end_date)
        .bind(payload.day_of_month)
        .bind(payload.next_run_date)
        .bind(payload.created_at)
        .bind(payload.updated_at)
        .fetch_one(&self.pool)
        .await?;

        let result = RecurrenceEntity {
            id: row.get("id"),
            account_id: row.get("account_id"),
            description: row.get("description"),
            amount: row.get("amount"),
            active: row.get("active"),
            start_date: row.get("start_date"),
            end_date: row.get("end_date"),
            day_of_month: row.get("day_of_month"),
            next_run_date: row.get("next_run_date"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

        Ok(Recurrence::from(result))
    }
}

mod entity {
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::debt::recurrence::Recurrence;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RecurrenceEntity {
        pub id: Uuid,
        pub account_id: Uuid,
        pub description: String,
        pub amount: Decimal,
        pub active: bool,
        pub start_date: NaiveDate,
        pub end_date: Option<NaiveDate>,
        pub day_of_month: i32,
        pub next_run_date: NaiveDate,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<Recurrence> for RecurrenceEntity {
        fn from(recurrence: Recurrence) -> Self {
            RecurrenceEntity {
                id: *recurrence.id(),
                account_id: *recurrence.account_id(),
                description: recurrence.description().to_string(),
                amount: *recurrence.amount(),
                active: *recurrence.active(),
                start_date: *recurrence.start_date(),
                end_date: *recurrence.end_date(),
                day_of_month: *recurrence.day_of_month(),
                next_run_date: *recurrence.next_run_date(),
                created_at: recurrence.created_at().naive_utc(),
                updated_at: recurrence.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<RecurrenceEntity> for Recurrence {
        fn from(entity: RecurrenceEntity) -> Self {
            Recurrence::from_row(
                entity.id,
                entity.account_id,
                entity.description,
                entity.amount,
                entity.active,
                entity.start_date,
                entity.end_date,
                entity.day_of_month,
                entity.next_run_date,
                entity.created_at.and_utc(),
                entity.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
