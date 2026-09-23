use async_trait::async_trait;
use chrono::Utc;
use http_error::{ext::OptionHttpExt, HttpError, HttpResult};
use rust_decimal::Decimal;
use sqlx::types::Json;
use sqlx::{Pool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

use util::DeletedBy;

use crate::modules::finance::{
    domain::{
        debt::Debt,
        payment::{Payment, PaymentFilters},
    },
    repository::debt::entity::DebtEntity,
};

#[async_trait]
pub trait PaymentRepository {
    async fn list(&self, filters: &PaymentFilters) -> HttpResult<Vec<Payment>>;

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<Payment>>;

    /// Persists `payment` and the `debt` it was already applied to (see
    /// `Debt::apply_payment`), re-syncing the installment parent if any.
    async fn register(&self, payment: Payment, debt: Debt) -> HttpResult<Payment>;

    /// Soft-deletes `payment` and persists the `debt` it was already reverted
    /// from (see `Debt::revert_payment`), re-syncing the installment parent if any.
    async fn refund(&self, payment: Payment, debt: Debt, deleted_by: DeletedBy) -> HttpResult<()>;
}

pub type DynPaymentRepository = dyn PaymentRepository + Send + Sync;

#[derive(Clone)]
pub struct PaymentRepositoryImpl {
    pool: Pool<Postgres>,
}

impl PaymentRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl PaymentRepository for PaymentRepositoryImpl {
    async fn list(&self, filters: &PaymentFilters) -> HttpResult<Vec<Payment>> {
        let mut builder =
            QueryBuilder::new("SELECT * FROM finance.payment WHERE deleted_by IS NULL");

        if let Some(client_id) = filters.client_id() {
            builder.push(" AND client_id = ");
            builder.push_bind(client_id);
        }

        if let Some(debt_ids) = filters.debt_ids() {
            builder.push(" AND debt_id = ANY(");
            builder.push_bind(debt_ids);
            builder.push(")");
        }

        if let Some(start_date) = filters.start_date() {
            builder.push(" AND payment_date >= ");
            builder.push_bind(start_date);
        }

        if let Some(end_date) = filters.end_date() {
            builder.push(" AND payment_date <= ");
            builder.push_bind(end_date);
        }

        builder.push(" ORDER BY payment_date DESC, created_at DESC");

        let rows = builder.build().fetch_all(&self.pool).await?;

        Ok(rows
            .iter()
            .map(|row| Payment::from(entity::PaymentEntity::from(row)))
            .collect())
    }

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<Payment>> {
        let row =
            sqlx::query(r#"SELECT * FROM finance.payment WHERE id = $1 AND deleted_by IS NULL"#)
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| Payment::from(entity::PaymentEntity::from(&r))))
    }

    async fn register(&self, payment: Payment, debt: Debt) -> HttpResult<Payment> {
        let mut tx = self.pool.begin().await?;

        let parent = lock_parent(&mut tx, &debt).await?;

        let paid_before = *debt.paid_amount() - *payment.amount();
        update_debt_balance(&mut tx, &debt, paid_before).await?;
        let inserted = insert_payment(&mut tx, payment).await?;

        if let Some(parent) = parent {
            sync_parent(&mut tx, parent).await?;
        }

        tx.commit().await?;
        Ok(inserted)
    }

    async fn refund(&self, payment: Payment, debt: Debt, deleted_by: DeletedBy) -> HttpResult<()> {
        let mut tx = self.pool.begin().await?;

        let parent = lock_parent(&mut tx, &debt).await?;

        let res = sqlx::query(
            r#"
            UPDATE finance.payment
            SET deleted_by = $1, updated_at = $2
            WHERE id = $3 AND deleted_by IS NULL
            "#,
        )
        .bind(Json(deleted_by))
        .bind(Utc::now().naive_utc())
        .bind(payment.id())
        .execute(&mut *tx)
        .await?;

        if res.rows_affected() == 0 {
            tx.rollback().await?;
            return Err(Box::new(HttpError::not_found("payment", payment.id())));
        }

        let paid_before = *debt.paid_amount() + *payment.amount();
        update_debt_balance(&mut tx, &debt, paid_before).await?;

        if let Some(parent) = parent {
            sync_parent(&mut tx, parent).await?;
        }

        tx.commit().await?;
        Ok(())
    }
}

/// Inserts a payment inside an existing transaction. Also used when a debt
/// is created with an initial paid amount.
pub(crate) async fn insert_payment(
    tx: &mut Transaction<'_, Postgres>,
    payment: Payment,
) -> HttpResult<Payment> {
    let dto = entity::PaymentEntity::from(payment);

    let row = sqlx::query(
        r#"
        INSERT INTO finance.payment (
            id,
            client_id,
            debt_id,
            amount,
            payment_date,
            created_at,
            updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(dto.id)
    .bind(dto.client_id)
    .bind(dto.debt_id)
    .bind(dto.amount)
    .bind(dto.payment_date)
    .bind(dto.created_at)
    .bind(dto.updated_at)
    .fetch_one(&mut **tx)
    .await?;

    Ok(Payment::from(entity::PaymentEntity::from(&row)))
}

/// Locks the installment parent of `debt` (if any) before its child is
/// touched, so concurrent payments on sibling installments serialize on the
/// parent and the lock order (parent, then child) matches the cascade delete.
async fn lock_parent(tx: &mut Transaction<'_, Postgres>, debt: &Debt) -> HttpResult<Option<Debt>> {
    let Some(parent_id) = debt.parent_id() else {
        return Ok(None);
    };

    let row = sqlx::query(
        r#"SELECT * FROM finance.debt WHERE id = $1 AND deleted_by IS NULL FOR UPDATE"#,
    )
    .bind(parent_id)
    .fetch_optional(&mut **tx)
    .await?
    .or_not_found("debt", parent_id.to_string())?;

    Ok(Some(Debt::from(DebtEntity::from(&row))))
}

/// Writes the debt's balance, failing with a conflict if its `paid_amount`
/// changed since it was read (a concurrent payment/refund won the race).
async fn update_debt_balance(
    tx: &mut Transaction<'_, Postgres>,
    debt: &Debt,
    expected_paid_amount: Decimal,
) -> HttpResult<()> {
    let res = sqlx::query(
        r#"
        UPDATE finance.debt
        SET paid_amount = $2, remaining_amount = $3, status = $4, updated_at = $5
        WHERE id = $1 AND paid_amount = $6 AND deleted_by IS NULL
        "#,
    )
    .bind(debt.id())
    .bind(debt.paid_amount())
    .bind(debt.remaining_amount())
    .bind(String::from(debt.status().clone()))
    .bind(debt.updated_at().map(|dt| dt.naive_utc()))
    .bind(expected_paid_amount)
    .execute(&mut **tx)
    .await?;

    if res.rows_affected() == 0 {
        return Err(Box::new(HttpError::conflict(
            "Debt was modified concurrently, please retry",
        )));
    }

    Ok(())
}

/// Recomputes the (already locked) parent's balance from its active children.
async fn sync_parent(tx: &mut Transaction<'_, Postgres>, mut parent: Debt) -> HttpResult<()> {
    let rows =
        sqlx::query(r#"SELECT * FROM finance.debt WHERE parent_id = $1 AND deleted_by IS NULL"#)
            .bind(parent.id())
            .fetch_all(&mut **tx)
            .await?;

    let children: Vec<Debt> = rows
        .iter()
        .map(|row| Debt::from(DebtEntity::from(row)))
        .collect();

    let paid_before = *parent.paid_amount();
    parent.sync_with_children(&children);
    update_debt_balance(tx, &parent, paid_before).await
}

pub mod entity {
    use chrono::{NaiveDate, NaiveDateTime};
    use rust_decimal::Decimal;
    use sqlx::postgres::PgRow;
    use sqlx::types::Json;
    use sqlx::Row;
    use uuid::Uuid;

    use util::DeletedBy;

    use crate::modules::finance::domain::payment::Payment;

    pub struct PaymentEntity {
        pub id: Uuid,
        pub client_id: Uuid,
        pub debt_id: Uuid,
        pub amount: Decimal,
        pub payment_date: NaiveDate,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
        pub deleted_by: Option<DeletedBy>,
    }

    impl From<&PgRow> for PaymentEntity {
        fn from(row: &PgRow) -> Self {
            Self {
                id: row.get("id"),
                client_id: row.get("client_id"),
                debt_id: row.get("debt_id"),
                amount: row.get("amount"),
                payment_date: row.get("payment_date"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                deleted_by: row
                    .get::<Option<Json<DeletedBy>>, _>("deleted_by")
                    .map(|j| j.0),
            }
        }
    }

    impl From<Payment> for PaymentEntity {
        fn from(payment: Payment) -> Self {
            Self {
                id: *payment.id(),
                client_id: *payment.client_id(),
                debt_id: *payment.debt_id(),
                amount: *payment.amount(),
                payment_date: *payment.payment_date(),
                created_at: payment.created_at().naive_utc(),
                updated_at: payment.updated_at().map(|dt| dt.naive_utc()),
                deleted_by: payment.deleted_by().clone(),
            }
        }
    }

    impl From<PaymentEntity> for Payment {
        fn from(dto: PaymentEntity) -> Self {
            Payment::from_row(
                dto.id,
                dto.client_id,
                dto.debt_id,
                dto.amount,
                dto.payment_date,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
                dto.deleted_by,
            )
        }
    }
}
