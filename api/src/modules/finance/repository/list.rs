use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::finance::domain::list::List;

#[async_trait]
pub trait ListRepository {
    async fn list(&self, client_id: Uuid) -> HttpResult<Vec<List>>;

    async fn insert(&self, list: List) -> HttpResult<List>;

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<List>>;

    async fn update(&self, list: List) -> HttpResult<List>;

    async fn delete(&self, id: Uuid) -> HttpResult<()>;
}

pub type DynListRepository = dyn ListRepository + Send + Sync;

#[derive(Clone)]
pub struct ListRepositoryImpl {
    pool: Pool<Postgres>,
}

impl ListRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl ListRepository for ListRepositoryImpl {
    async fn list(&self, client_id: Uuid) -> HttpResult<Vec<List>> {
        let rows = sqlx::query(
            r#"SELECT * FROM finance.list WHERE client_id = $1 ORDER BY created_at ASC"#,
        )
        .bind(client_id)
        .fetch_all(&self.pool)
        .await?;

        let lists: Vec<List> = rows
            .into_iter()
            .map(|row| List::from(entity::ListEntity::from(&row)))
            .collect();
        Ok(lists)
    }

    async fn insert(&self, list: List) -> HttpResult<List> {
        let list_dto = entity::ListEntity::from(list);

        let row = sqlx::query(
            r#"
            INSERT INTO finance.list (id, client_id, name, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(list_dto.id)
        .bind(list_dto.client_id)
        .bind(&list_dto.name)
        .bind(list_dto.created_at)
        .bind(list_dto.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(List::from(entity::ListEntity::from(&row)))
    }

    async fn get_by_id(&self, id: &Uuid) -> HttpResult<Option<List>> {
        let row = sqlx::query(r#"SELECT * FROM finance.list WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| List::from(entity::ListEntity::from(&r))))
    }

    async fn update(&self, list: List) -> HttpResult<List> {
        let list_dto = entity::ListEntity::from(list);

        let row = sqlx::query(
            r#"
            UPDATE finance.list SET
                name = $2,
                updated_at = $3
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(list_dto.id)
        .bind(&list_dto.name)
        .bind(list_dto.updated_at)
        .fetch_optional(&self.pool)
        .await?
        .or_not_found("list", list_dto.id.to_string())?;

        Ok(List::from(entity::ListEntity::from(&row)))
    }

    async fn delete(&self, id: Uuid) -> HttpResult<()> {
        let result = sqlx::query(r#"DELETE FROM finance.list WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(Box::new(http_error::HttpError::not_found("list", id)));
        }

        Ok(())
    }
}

pub mod entity {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Serialize};
    use sqlx::postgres::PgRow;
    use sqlx::Row;
    use uuid::Uuid;

    use crate::modules::finance::domain::list::List;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ListEntity {
        pub id: Uuid,
        pub client_id: Uuid,
        pub name: String,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<&PgRow> for ListEntity {
        fn from(row: &PgRow) -> Self {
            Self {
                id: row.get("id"),
                client_id: row.get("client_id"),
                name: row.get("name"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        }
    }

    impl From<List> for ListEntity {
        fn from(list: List) -> Self {
            ListEntity {
                id: *list.id(),
                client_id: *list.client_id(),
                name: list.name().clone(),
                created_at: list.created_at().naive_utc(),
                updated_at: list.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<ListEntity> for List {
        fn from(dto: ListEntity) -> Self {
            List::from_row(
                dto.id,
                dto.client_id,
                dto.name,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
