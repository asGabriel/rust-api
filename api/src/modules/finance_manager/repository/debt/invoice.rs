use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{types::Json, Pool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::modules::{
    finance_manager::domain::debt::invoice::{filters::InvoiceFilters, Invoice},
    shared::repository::Repository,
};

pub type DynInvoiceRepository = dyn Repository<Invoice, InvoiceFilters, Uuid> + Send + Sync;

pub struct InvoiceRepositoryImpl {
    pool: Pool<Postgres>,
}

impl InvoiceRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl Repository<Invoice, InvoiceFilters, Uuid> for InvoiceRepositoryImpl {
    async fn list(&self, filters: &InvoiceFilters) -> HttpResult<Vec<Invoice>> {
        let mut builder =
            QueryBuilder::new("SELECT * FROM finance_manager.invoice WHERE client_id = ");
        builder.push_bind(filters.client_id);

        if let Some(debt_ids) = &filters.related_debt_ids {
            builder.push(" AND related_debt_ids && ");
            builder.push_bind(debt_ids);
        }

        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        Ok(rows.iter().map(Invoice::from).collect())
    }

    async fn get(&self, id: &Uuid) -> HttpResult<Option<Invoice>> {
        let row = sqlx::query(r#"SELECT * FROM finance_manager.invoice WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.as_ref().map(Invoice::from))
    }

    async fn insert(&self, item: Invoice) -> HttpResult<Invoice> {
        let deleted_by = item.deleted_by().clone().map(Json);

        let row = sqlx::query(
            r#"
            INSERT INTO finance_manager.invoice (
                id,
                client_id,
                name,
                related_debt_ids,
                created_at,
                updated_at,
                deleted_by
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
            "#,
        )
        .bind(*item.id())
        .bind(*item.client_id())
        .bind(item.name())
        .bind(Vec::from_iter(item.related_debt_ids().iter().copied()))
        .bind(*item.created_at())
        .bind(*item.updated_at())
        .bind(deleted_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(Invoice::from(&row))
    }

    async fn insert_many(&self, items: Vec<Invoice>) -> HttpResult<Vec<Invoice>> {
        let mut tx = self.pool.begin().await?;
        let mut results = Vec::with_capacity(items.len());

        for item in items {
            let deleted_by = item.deleted_by().clone().map(Json);

            let row = sqlx::query(
                r#"
                INSERT INTO finance_manager.invoice (
                    id,
                    client_id,
                    name,
                    related_debt_ids,
                    created_at,
                    updated_at,
                    deleted_by
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING *
                "#,
            )
            .bind(*item.id())
            .bind(*item.client_id())
            .bind(item.name())
            .bind(Vec::from_iter(item.related_debt_ids().iter().copied()))
            .bind(*item.created_at())
            .bind(*item.updated_at())
            .bind(deleted_by)
            .fetch_one(&mut *tx)
            .await?;

            results.push(Invoice::from(&row));
        }

        tx.commit().await?;
        Ok(results)
    }

    async fn update(&self, item: Invoice) -> HttpResult<Invoice> {
        let deleted_by = item.deleted_by().clone().map(Json);

        let row = sqlx::query(
            r#"
            UPDATE finance_manager.invoice SET
                client_id = $2,
                name = $3,
                related_debt_ids = $4,
                updated_at = $5,
                deleted_by = $6
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(*item.id())
        .bind(*item.client_id())
        .bind(item.name())
        .bind(Vec::from_iter(item.related_debt_ids().iter().copied()))
        .bind(*item.updated_at())
        .bind(deleted_by)
        .fetch_one(&self.pool)
        .await?;

        Ok(Invoice::from(&row))
    }

    async fn delete(&self, _id: &Uuid) -> HttpResult<()> {
        unimplemented!()
    }
}
