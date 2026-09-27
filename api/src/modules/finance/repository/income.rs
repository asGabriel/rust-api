use async_trait::async_trait;
use chrono::Utc;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use sqlx::types::Json;
use sqlx::{Pool, Postgres, QueryBuilder};
use uuid::Uuid;

use util::DeletedBy;

use crate::modules::finance::domain::income::{Income, IncomeFilters};

#[async_trait]
pub trait IncomeRepository {
    async fn list(&self, filters: &IncomeFilters) -> HttpResult<Vec<Income>>;

    async fn insert(&self, income: Income) -> HttpResult<Income>;

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<Income>>;

    async fn update(&self, income: Income) -> HttpResult<Income>;

    async fn soft_delete(&self, id: Uuid, deleted_by: DeletedBy) -> HttpResult<()>;
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
    async fn list(&self, filters: &IncomeFilters) -> HttpResult<Vec<Income>> {
        let mut builder =
            QueryBuilder::new("SELECT * FROM finance.income WHERE deleted_by IS NULL");

        if let Some(client_id) = filters.client_id() {
            builder.push(" AND client_id = ");
            builder.push_bind(client_id);
        }

        if let Some(categories) = filters.categories() {
            builder.push(" AND category = ANY(");
            builder.push_bind(
                categories
                    .iter()
                    .cloned()
                    .map(String::from)
                    .collect::<Vec<String>>(),
            );
            builder.push(")");
        }

        if let Some(start_date) = filters.start_date() {
            builder.push(" AND received_date >= ");
            builder.push_bind(start_date);
        }

        if let Some(end_date) = filters.end_date() {
            builder.push(" AND received_date <= ");
            builder.push_bind(end_date);
        }

        builder.push(" ORDER BY received_date ASC, created_at ASC");

        let rows = builder.build().fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(|row| Income::from(entity::IncomeEntity::from(&row)))
            .collect())
    }

    async fn insert(&self, income: Income) -> HttpResult<Income> {
        let income_dto = entity::IncomeEntity::from(income);

        let row = sqlx::query(
            r#"
            INSERT INTO finance.income (
                id,
                client_id,
                category,
                description,
                amount,
                received_date,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *
            "#,
        )
        .bind(income_dto.id)
        .bind(income_dto.client_id)
        .bind(&income_dto.category)
        .bind(&income_dto.description)
        .bind(income_dto.amount)
        .bind(income_dto.received_date)
        .bind(income_dto.created_at)
        .bind(income_dto.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(Income::from(entity::IncomeEntity::from(&row)))
    }

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<Income>> {
        let row =
            sqlx::query(r#"SELECT * FROM finance.income WHERE id = $1 AND deleted_by IS NULL"#)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| Income::from(entity::IncomeEntity::from(&r))))
    }

    async fn update(&self, income: Income) -> HttpResult<Income> {
        let income_dto = entity::IncomeEntity::from(income);

        let row = sqlx::query(
            r#"
            UPDATE finance.income SET
                category = $2,
                description = $3,
                amount = $4,
                received_date = $5,
                updated_at = $6
            WHERE id = $1 AND deleted_by IS NULL
            RETURNING *
            "#,
        )
        .bind(income_dto.id)
        .bind(&income_dto.category)
        .bind(&income_dto.description)
        .bind(income_dto.amount)
        .bind(income_dto.received_date)
        .bind(income_dto.updated_at)
        .fetch_optional(&self.pool)
        .await?
        .or_not_found("income", income_dto.id.to_string())?;

        Ok(Income::from(entity::IncomeEntity::from(&row)))
    }

    async fn soft_delete(&self, id: Uuid, deleted_by: DeletedBy) -> HttpResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE finance.income
            SET deleted_by = $1, updated_at = $2
            WHERE id = $3 AND deleted_by IS NULL
            "#,
        )
        .bind(Json(deleted_by))
        .bind(Utc::now().naive_utc())
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(Box::new(HttpError::not_found("income", id)));
        }

        Ok(())
    }
}

pub mod entity {
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::PgRow;
    use sqlx::types::Json;
    use sqlx::Row;
    use uuid::Uuid;

    use util::DeletedBy;

    use crate::modules::finance::domain::income::{Income, IncomeCategory};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IncomeEntity {
        pub id: Uuid,
        pub client_id: Uuid,
        pub category: String,
        pub description: String,
        pub amount: Decimal,
        pub received_date: NaiveDate,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
        pub deleted_by: Option<DeletedBy>,
    }

    impl From<&PgRow> for IncomeEntity {
        fn from(row: &PgRow) -> Self {
            Self {
                id: row.get("id"),
                client_id: row.get("client_id"),
                category: row.get("category"),
                description: row.get("description"),
                amount: row.get("amount"),
                received_date: row.get("received_date"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                deleted_by: row
                    .get::<Option<Json<DeletedBy>>, _>("deleted_by")
                    .map(|j| j.0),
            }
        }
    }

    impl From<Income> for IncomeEntity {
        fn from(income: Income) -> Self {
            IncomeEntity {
                id: *income.id(),
                client_id: *income.client_id(),
                category: String::from(income.category().clone()),
                description: income.description().clone(),
                amount: *income.amount(),
                received_date: *income.received_date(),
                created_at: income.created_at().naive_utc(),
                updated_at: income.updated_at().map(|dt| dt.naive_utc()),
                deleted_by: income.deleted_by().clone(),
            }
        }
    }

    impl From<IncomeEntity> for Income {
        fn from(dto: IncomeEntity) -> Self {
            Income::from_row(
                dto.id,
                dto.client_id,
                IncomeCategory::from(dto.category),
                dto.description,
                dto.amount,
                dto.received_date,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
                dto.deleted_by,
            )
        }
    }
}
