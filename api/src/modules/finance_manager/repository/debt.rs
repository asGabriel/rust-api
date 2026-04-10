use async_trait::async_trait;
use chrono::Utc;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use sqlx::types::Json;
use sqlx::{Pool, Postgres, QueryBuilder};
use uuid::Uuid;

use util::DeletedBy;

use crate::modules::finance_manager::domain::debt::{Debt, DebtFilters};

pub mod installment;
pub mod invoice;

#[async_trait]
pub trait DebtRepository {
    async fn list(&self, filters: &DebtFilters) -> HttpResult<Vec<Debt>>;

    async fn insert(&self, debt: Debt) -> HttpResult<Debt>;

    async fn get_by_identification(&self, identification: &str) -> HttpResult<Option<Debt>>;

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<Debt>>;

    async fn update(&self, debt: Debt) -> HttpResult<Debt>;

    async fn soft_delete_cascade(
        &self,
        client_id: Uuid,
        debt_id: Uuid,
        deleted_by: DeletedBy,
    ) -> HttpResult<()>;
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
                expense_type = $3,
                tags = $4,
                description = $5, 
                total_amount = $6, 
                paid_amount = $7, 
                discount_amount = $8, 
                remaining_amount = $9, 
                due_date = $10, 
                status = $11, 
                installment_count = $12,
                financial_instrument_id = $13,
                updated_at = $14
            WHERE id = $1 
            RETURNING *
            "#,
        )
        .bind(debt_dto.id)
        .bind(&debt_dto.category)
        .bind(&debt_dto.expense_type)
        .bind(&debt_dto.tags)
        .bind(&debt_dto.description)
        .bind(debt_dto.total_amount)
        .bind(debt_dto.paid_amount)
        .bind(debt_dto.discount_amount)
        .bind(debt_dto.remaining_amount)
        .bind(debt_dto.due_date)
        .bind(&debt_dto.status)
        .bind(debt_dto.installment_count)
        .bind(debt_dto.financial_instrument_id)
        .bind(debt_dto.updated_at)
        .fetch_optional(&self.pool)
        .await?
        .or_not_found("debt", debt_dto.id.to_string())?;

        Ok(Debt::from(entity::DebtEntity::from(&row)))
    }

    async fn soft_delete_cascade(
        &self,
        client_id: Uuid,
        debt_id: Uuid,
        deleted_by: DeletedBy,
    ) -> HttpResult<()> {
        let mut tx = self.pool.begin().await?;
        let now = Utc::now().naive_utc();
        let meta = Json(deleted_by.clone());

        let debt_res = sqlx::query(
            r#"
            UPDATE finance_manager.debt
            SET deleted_by = $1, updated_at = $2
            WHERE id = $3 AND client_id = $4 AND deleted_by IS NULL
            "#,
        )
        .bind(&meta)
        .bind(now)
        .bind(debt_id)
        .bind(client_id)
        .execute(&mut *tx)
        .await?;

        if debt_res.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(Box::new(HttpError::not_found("debt", debt_id)));
        }

        let meta = Json(deleted_by.clone());
        sqlx::query(
            r#"
            UPDATE finance_manager.payment
            SET deleted_by = $1, updated_at = $2
            WHERE debt_id = $3 AND deleted_by IS NULL
            "#,
        )
        .bind(&meta)
        .bind(now)
        .bind(debt_id)
        .execute(&mut *tx)
        .await?;

        let meta = Json(deleted_by);
        sqlx::query(
            r#"
            UPDATE finance_manager.debt_installment
            SET deleted_by = $1, updated_at = $2
            WHERE debt_id = $3 AND deleted_by IS NULL
            "#,
        )
        .bind(&meta)
        .bind(now)
        .bind(debt_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<Debt>> {
        let row = sqlx::query(
            r#"SELECT * FROM finance_manager.debt WHERE id = $1 AND deleted_by IS NULL"#,
        )
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

        let row = sqlx::query(
            r#"SELECT * FROM finance_manager.debt WHERE identification = $1 AND deleted_by IS NULL"#,
        )
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
                client_id,
                category,
                expense_type,
                tags,
                description, 
                total_amount, 
                paid_amount, 
                discount_amount, 
                remaining_amount, 
                due_date,
                status,
                installment_count,
                financial_instrument_id,
                created_at,
                updated_at
            ) 
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            RETURNING *
        "#,
        )
        .bind(debt_dto.id)
        .bind(debt_dto.client_id)
        .bind(&debt_dto.category)
        .bind(&debt_dto.expense_type)
        .bind(&debt_dto.tags)
        .bind(&debt_dto.description)
        .bind(debt_dto.total_amount)
        .bind(debt_dto.paid_amount)
        .bind(debt_dto.discount_amount)
        .bind(debt_dto.remaining_amount)
        .bind(debt_dto.due_date)
        .bind(&debt_dto.status)
        .bind(debt_dto.installment_count)
        .bind(debt_dto.financial_instrument_id)
        .bind(debt_dto.created_at)
        .bind(debt_dto.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(Debt::from(entity::DebtEntity::from(&row)))
    }

    async fn list(&self, filters: &DebtFilters) -> HttpResult<Vec<Debt>> {
        let mut builder =
            QueryBuilder::new("SELECT * FROM finance_manager.debt WHERE deleted_by IS NULL");

        if let Some(ids) = filters.ids() {
            builder.push(" AND id = ANY(");
            builder.push_bind(ids);
            builder.push(")");
        }

        if let Some(client_id) = filters.client_id() {
            builder.push(" AND client_id = ");
            builder.push_bind(client_id);
        }

        if let Some(start_date) = filters.start_date() {
            builder.push(" AND due_date >= ");
            builder.push_bind(start_date);
        }

        if let Some(end_date) = filters.end_date() {
            builder.push(" AND due_date <= ");
            builder.push_bind(end_date);
        }

        if let Some(category_names) = filters.category_names() {
            builder.push(" AND category = ANY(");
            builder.push_bind(category_names);
            builder.push(")");
        }

        if let Some(statuses) = filters.statuses() {
            builder.push(" AND status = ANY(");
            builder.push_bind(
                statuses
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>(),
            );
            builder.push(")");
        }

        if let Some(ids) = filters.financial_instrument_ids() {
            builder.push(" AND financial_instrument_id = ANY(");
            builder.push_bind(ids);
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

    use sqlx::types::Json;

    use util::DeletedBy;

    use crate::modules::finance_manager::domain::debt::{Debt, DebtCategory, ExpenseType};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DebtEntity {
        pub id: Uuid,
        pub client_id: Uuid,
        pub identification: String,
        pub category: String,
        pub expense_type: String,
        pub tags: Vec<String>,
        pub description: String,
        pub total_amount: Decimal,
        pub paid_amount: Decimal,
        pub discount_amount: Decimal,
        pub remaining_amount: Decimal,
        pub due_date: NaiveDate,
        pub status: String,
        pub installment_count: Option<i32>,
        pub financial_instrument_id: Option<Uuid>,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
        pub deleted_by: Option<DeletedBy>,
    }

    impl From<&PgRow> for DebtEntity {
        fn from(row: &PgRow) -> Self {
            Self {
                id: row.get("id"),
                client_id: row.get("client_id"),
                identification: row.get::<i32, _>("identification").to_string(),
                category: row.get::<String, _>("category"),
                expense_type: row.get::<String, _>("expense_type"),
                tags: row.get::<Vec<String>, _>("tags"),
                description: row.get("description"),
                total_amount: row.get("total_amount"),
                paid_amount: row.get("paid_amount"),
                discount_amount: row.get("discount_amount"),
                remaining_amount: row.get("remaining_amount"),
                due_date: row.get("due_date"),
                status: row.get("status"),
                installment_count: row.get("installment_count"),
                financial_instrument_id: row.get("financial_instrument_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                deleted_by: row
                    .get::<Option<Json<DeletedBy>>, _>("deleted_by")
                    .map(|j| j.0),
            }
        }
    }

    impl From<Debt> for DebtEntity {
        fn from(debt: Debt) -> Self {
            DebtEntity {
                id: *debt.id(),
                client_id: *debt.client_id(),
                identification: debt.identification().to_string(),
                category: String::from(debt.category().clone()),
                expense_type: debt.expense_type().as_str().to_string(),
                tags: debt.tags().clone(),
                description: debt.description().clone(),
                total_amount: *debt.total_amount(),
                paid_amount: *debt.paid_amount(),
                discount_amount: *debt.discount_amount(),
                remaining_amount: *debt.remaining_amount(),
                due_date: *debt.due_date(),
                status: debt.status().clone().into(),
                installment_count: *debt.installment_count(),
                financial_instrument_id: *debt.financial_instrument_id(),
                created_at: debt.created_at().naive_utc(),
                updated_at: debt.updated_at().map(|dt| dt.naive_utc()),
                deleted_by: debt.deleted_by().clone(),
            }
        }
    }

    impl From<DebtEntity> for Debt {
        fn from(dto: DebtEntity) -> Self {
            Debt::from_row(
                dto.id,
                dto.client_id,
                DebtCategory::from(dto.category),
                ExpenseType::from_str(&dto.expense_type),
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
                dto.financial_instrument_id,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
                dto.deleted_by,
            )
        }
    }
}
