use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};
use sqlx::{Pool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::modules::finance_manager::domain::debt::{Debt, DebtFilters};

pub mod installment;

#[async_trait]
pub trait DebtRepository {
    async fn list(&self, filters: &DebtFilters) -> HttpResult<Vec<Debt>>;

    async fn insert(&self, debt: Debt) -> HttpResult<Debt>;

    async fn get_by_identification(&self, identification: &str) -> HttpResult<Option<Debt>>;

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<Debt>>;

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

        let row = sqlx::query(
            r#"
            UPDATE finance_manager.debt SET 
                category = $2,
                tags = $3,
                description = $4, 
                total_amount = $5, 
                paid_amount = $6, 
                discount_amount = $7, 
                remaining_amount = $8, 
                due_date = $9, 
                status = $10, 
                installment_count = $11,
                updated_at = $12
            WHERE id = $1 
            RETURNING *
            "#,
        )
        .bind(debt_dto.id)
        .bind(&debt_dto.category)
        .bind(&debt_dto.tags)
        .bind(&debt_dto.description)
        .bind(debt_dto.total_amount)
        .bind(debt_dto.paid_amount)
        .bind(debt_dto.discount_amount)
        .bind(debt_dto.remaining_amount)
        .bind(debt_dto.due_date)
        .bind(&debt_dto.status)
        .bind(debt_dto.installment_count)
        .bind(debt_dto.updated_at)
        .fetch_optional(&self.pool)
        .await?
        .or_not_found("debt", debt_dto.id.to_string())?;

        Ok(Debt::from(entity::DebtEntity::from(&row)))
    }

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<Debt>> {
        let row = sqlx::query(r#"SELECT * FROM finance_manager.debt WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| Debt::from(entity::DebtEntity::from(&r))))
    }

    async fn get_by_identification(&self, identification: &str) -> HttpResult<Option<Debt>> {
        let identification_num: i32 = identification.parse().map_err(|_| {
            http_error::HttpError::bad_request(format!(
                "Invalid identification format: {}",
                identification
            ))
        })?;

        let row = sqlx::query(r#"SELECT * FROM finance_manager.debt WHERE identification = $1"#)
            .bind(identification_num)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| Debt::from(entity::DebtEntity::from(&r))))
    }

    async fn insert(&self, debt: Debt) -> HttpResult<Debt> {
        let debt_dto = entity::DebtEntity::from(debt);

        let row = sqlx::query(
            r#"
            INSERT INTO finance_manager.debt (
                id, 
                category,
                tags,
                description, 
                total_amount, 
                paid_amount, 
                discount_amount, 
                remaining_amount, 
                due_date,
                status,
                installment_count,
                created_at,
                updated_at
            ) 
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING *
        "#,
        )
        .bind(debt_dto.id)
        .bind(&debt_dto.category)
        .bind(&debt_dto.tags)
        .bind(&debt_dto.description)
        .bind(debt_dto.total_amount)
        .bind(debt_dto.paid_amount)
        .bind(debt_dto.discount_amount)
        .bind(debt_dto.remaining_amount)
        .bind(debt_dto.due_date)
        .bind(debt_dto.status)
        .bind(debt_dto.installment_count)
        .bind(debt_dto.created_at)
        .bind(debt_dto.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(Debt::from(entity::DebtEntity::from(&row)))
    }

    async fn list(&self, filters: &DebtFilters) -> HttpResult<Vec<Debt>> {
        let mut builder = QueryBuilder::new("SELECT * FROM finance_manager.debt");
        let mut has_where = false;

        if let Some(start_date) = filters.start_date() {
            builder.push(if has_where { " AND " } else { " WHERE " });
            builder.push("due_date >= ");
            builder.push_bind(start_date);
            has_where = true;
        }

        if let Some(end_date) = filters.end_date() {
            builder.push(if has_where { " AND " } else { " WHERE " });
            builder.push("due_date <= ");
            builder.push_bind(end_date);
            has_where = true;
        }

        if let Some(category_names) = filters.category_names() {
            builder.push(if has_where { " AND " } else { " WHERE " });
            builder.push("category = ANY(");
            builder.push_bind(category_names);
            builder.push(")");
            has_where = true;
        }

        if let Some(statuses) = filters.statuses() {
            builder.push(if has_where { " AND " } else { " WHERE " });
            builder.push("status = ANY(");
            builder.push_bind(
                statuses
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            );
            builder.push(")");
        }

        builder.push(" ORDER BY due_date ASC, status DESC");

        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        let debts: Vec<Debt> = rows
            .into_iter()
            .map(|row| Debt::from(entity::DebtEntity::from(&row)))
            .collect();
        Ok(debts)
    }
}

pub mod entity {
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal::Decimal;
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::PgRow;
    use sqlx::Row;
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::debt::{Debt, DebtCategory};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DebtEntity {
        pub id: Uuid,
        pub identification: String,
        pub category: String,
        pub tags: Vec<String>,
        pub description: String,
        pub total_amount: Decimal,
        pub paid_amount: Decimal,
        pub discount_amount: Decimal,
        pub remaining_amount: Decimal,
        pub due_date: NaiveDate,
        pub status: String,
        pub installment_count: Option<i32>,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<&PgRow> for DebtEntity {
        fn from(row: &PgRow) -> Self {
            Self {
                id: row.get("id"),
                identification: row.get::<i32, _>("identification").to_string(),
                category: row.get::<String, _>("category"),
                tags: row.get::<Vec<String>, _>("tags"),
                description: row.get("description"),
                total_amount: row.get("total_amount"),
                paid_amount: row.get("paid_amount"),
                discount_amount: row.get("discount_amount"),
                remaining_amount: row.get("remaining_amount"),
                due_date: row.get("due_date"),
                status: row.get("status"),
                installment_count: row.get("installment_count"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        }
    }

    impl From<Debt> for DebtEntity {
        fn from(debt: Debt) -> Self {
            DebtEntity {
                id: *debt.id(),
                identification: debt.identification().to_string(),
                category: String::from(debt.category().clone()),
                tags: debt.tags().clone(),
                description: debt.description().clone(),
                total_amount: *debt.total_amount(),
                paid_amount: *debt.paid_amount(),
                discount_amount: *debt.discount_amount(),
                remaining_amount: *debt.remaining_amount(),
                due_date: *debt.due_date(),
                status: debt.status().clone().into(),
                installment_count: *debt.installment_count(),
                created_at: debt.created_at().naive_utc(),
                updated_at: debt.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<DebtEntity> for Debt {
        fn from(dto: DebtEntity) -> Self {
            Debt::from_row(
                dto.id,
                DebtCategory::from(dto.category),
                dto.tags,
                dto.identification,
                dto.description,
                dto.total_amount,
                dto.paid_amount,
                dto.discount_amount,
                dto.remaining_amount,
                dto.due_date,
                dto.status.into(),
                dto.installment_count,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
