use async_trait::async_trait;
use chrono::Utc;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use sqlx::types::Json;
use sqlx::{Pool, Postgres, QueryBuilder};
use uuid::Uuid;

use util::DeletedBy;

use crate::modules::finance::{
    domain::{
        debt::{Debt, DebtFilters},
        payment::Payment,
    },
    repository::payment::insert_payment,
};

#[async_trait]
pub trait DebtRepository {
    async fn list(&self, filters: &DebtFilters) -> HttpResult<Vec<Debt>>;

    /// Inserts `debt`, plus its `initial_payment` (if any) in the same transaction.
    async fn insert(&self, debt: Debt, initial_payment: Option<Payment>) -> HttpResult<Debt>;

    async fn insert_many(&self, debts: Vec<Debt>) -> HttpResult<Vec<Debt>>;

    async fn get_by_identification(&self, identification: &str) -> HttpResult<Option<Debt>>;

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<Debt>>;

    async fn update(&self, debt: Debt) -> HttpResult<Debt>;

    /// Updates the debt and copies its shared fields (`category`,
    /// `expense_type`, `list_id`, `description`) to its installment children,
    /// atomically.
    async fn update_with_children(&self, debt: Debt) -> HttpResult<Debt>;

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
        let mut tx = self.pool.begin().await?;
        let updated = update_one(&mut tx, debt).await?;
        tx.commit().await?;
        Ok(updated)
    }

    async fn update_with_children(&self, debt: Debt) -> HttpResult<Debt> {
        let mut tx = self.pool.begin().await?;
        let updated = update_one(&mut tx, debt).await?;
        let updated_dto = entity::DebtEntity::from(updated.clone());

        sqlx::query(
            r#"
            UPDATE finance.debt SET
                category = $1,
                expense_type = $2,
                list_id = $3,
                description = $4,
                updated_at = $5
            WHERE parent_id = $6 AND deleted_by IS NULL
            "#,
        )
        .bind(&updated_dto.category)
        .bind(&updated_dto.expense_type)
        .bind(updated_dto.list_id)
        .bind(&updated_dto.description)
        .bind(Utc::now().naive_utc())
        .bind(updated_dto.id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated)
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
            UPDATE finance.debt
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

        sqlx::query(
            r#"
            UPDATE finance.debt
            SET deleted_by = $1, updated_at = $2
            WHERE parent_id = $3 AND deleted_by IS NULL
            "#,
        )
        .bind(&meta)
        .bind(now)
        .bind(debt_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            UPDATE finance.payment
            SET deleted_by = $1, updated_at = $2
            WHERE deleted_by IS NULL
              AND debt_id IN (SELECT id FROM finance.debt WHERE id = $3 OR parent_id = $3)
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
        let row = sqlx::query(r#"SELECT * FROM finance.debt WHERE id = $1 AND deleted_by IS NULL"#)
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
            r#"SELECT * FROM finance.debt WHERE identification = $1 AND deleted_by IS NULL"#,
        )
        .bind(identification_num)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Debt::from(entity::DebtEntity::from(&r))))
    }

    async fn insert(&self, debt: Debt, initial_payment: Option<Payment>) -> HttpResult<Debt> {
        let mut tx = self.pool.begin().await?;
        let inserted = insert_one(&mut tx, debt).await?;
        if let Some(payment) = initial_payment {
            insert_payment(&mut tx, payment).await?;
        }
        tx.commit().await?;
        Ok(inserted)
    }

    async fn insert_many(&self, debts: Vec<Debt>) -> HttpResult<Vec<Debt>> {
        let mut tx = self.pool.begin().await?;
        let mut results = Vec::with_capacity(debts.len());

        for debt in debts {
            results.push(insert_one(&mut tx, debt).await?);
        }

        tx.commit().await?;
        Ok(results)
    }

    async fn list(&self, filters: &DebtFilters) -> HttpResult<Vec<Debt>> {
        let mut builder = QueryBuilder::new("SELECT * FROM finance.debt WHERE deleted_by IS NULL");

        if let Some(ids) = filters.ids() {
            builder.push(" AND id = ANY(");
            builder.push_bind(ids);
            builder.push(")");
        }

        if let Some(client_id) = filters.client_id() {
            builder.push(" AND client_id = ");
            builder.push_bind(client_id);
        }

        match filters.parent_id() {
            Some(parent_id) => {
                builder.push(" AND parent_id = ");
                builder.push_bind(parent_id);
            }
            None if !*filters.include_children() => {
                builder.push(" AND parent_id IS NULL");
            }
            None => {}
        }

        if let Some(start_date) = filters.start_date() {
            builder.push(" AND due_date >= ");
            builder.push_bind(start_date);
        }

        if let Some(end_date) = filters.end_date() {
            builder.push(" AND due_date <= ");
            builder.push_bind(end_date);
        }

        if let Some(list_id) = filters.list_id() {
            builder.push(" AND list_id = ");
            builder.push_bind(list_id);
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

/// Updates the editable fields only. Amounts and status are never written
/// here — they only move through payments/refunds.
async fn update_one(tx: &mut sqlx::Transaction<'_, Postgres>, debt: Debt) -> HttpResult<Debt> {
    let debt_dto = entity::DebtEntity::from(debt);

    let row = sqlx::query(
        r#"
        UPDATE finance.debt SET
            category = $2,
            expense_type = $3,
            list_id = $4,
            description = $5,
            due_date = $6,
            updated_at = $7
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(debt_dto.id)
    .bind(&debt_dto.category)
    .bind(&debt_dto.expense_type)
    .bind(debt_dto.list_id)
    .bind(&debt_dto.description)
    .bind(debt_dto.due_date)
    .bind(debt_dto.updated_at)
    .fetch_optional(&mut **tx)
    .await?
    .or_not_found("debt", debt_dto.id.to_string())?;

    Ok(Debt::from(entity::DebtEntity::from(&row)))
}

async fn insert_one(tx: &mut sqlx::Transaction<'_, Postgres>, debt: Debt) -> HttpResult<Debt> {
    let debt_dto = entity::DebtEntity::from(debt);

    let row = sqlx::query(
        r#"
        INSERT INTO finance.debt (
            id,
            client_id,
            category,
            expense_type,
            list_id,
            description,
            total_amount,
            paid_amount,
            remaining_amount,
            due_date,
            status,
            installment_count,
            parent_id,
            installment_number,
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
    .bind(debt_dto.list_id)
    .bind(&debt_dto.description)
    .bind(debt_dto.total_amount)
    .bind(debt_dto.paid_amount)
    .bind(debt_dto.remaining_amount)
    .bind(debt_dto.due_date)
    .bind(&debt_dto.status)
    .bind(debt_dto.installment_count)
    .bind(debt_dto.parent_id)
    .bind(debt_dto.installment_number)
    .bind(debt_dto.created_at)
    .bind(debt_dto.updated_at)
    .fetch_one(&mut **tx)
    .await?;

    Ok(Debt::from(entity::DebtEntity::from(&row)))
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

    use crate::modules::finance::domain::debt::{Debt, DebtCategory, ExpenseType};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct DebtEntity {
        pub id: Uuid,
        pub client_id: Uuid,
        pub identification: String,
        pub category: String,
        pub expense_type: String,
        pub list_id: Option<Uuid>,
        pub description: String,
        pub total_amount: Decimal,
        pub paid_amount: Decimal,
        pub remaining_amount: Decimal,
        pub due_date: Option<NaiveDate>,
        pub status: String,
        pub installment_count: Option<i32>,
        pub parent_id: Option<Uuid>,
        pub installment_number: Option<i32>,
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
                list_id: row.get("list_id"),
                description: row.get("description"),
                total_amount: row.get("total_amount"),
                paid_amount: row.get("paid_amount"),
                remaining_amount: row.get("remaining_amount"),
                due_date: row.get("due_date"),
                status: row.get("status"),
                installment_count: row.get("installment_count"),
                parent_id: row.get("parent_id"),
                installment_number: row.get("installment_number"),
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
                list_id: *debt.list_id(),
                description: debt.description().clone(),
                total_amount: *debt.total_amount(),
                paid_amount: *debt.paid_amount(),
                remaining_amount: *debt.remaining_amount(),
                due_date: *debt.due_date(),
                status: debt.status().clone().into(),
                installment_count: *debt.installment_count(),
                parent_id: *debt.parent_id(),
                installment_number: *debt.installment_number(),
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
                dto.list_id,
                dto.identification,
                dto.description,
                dto.total_amount,
                dto.paid_amount,
                dto.remaining_amount,
                dto.due_date,
                dto.status.into(),
                dto.installment_count,
                dto.parent_id,
                dto.installment_number,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
                dto.deleted_by,
            )
        }
    }
}
