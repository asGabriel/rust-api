use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, Row};

use crate::modules::finance_manager::{
    domain::debt::category::DebtCategory, repository::debt::category::entity::DebtCategoryEntity,
};

pub type DynDebtCategoryRepository = dyn DebtCategoryRepository + Send + Sync;

#[async_trait]
pub trait DebtCategoryRepository {
    async fn list(&self) -> HttpResult<Vec<DebtCategory>>;
    async fn insert(&self, category: DebtCategory) -> HttpResult<DebtCategory>;
    async fn get_by_name(&self, name: &str) -> HttpResult<Option<DebtCategory>>;
}

pub struct DebtCategoryRepositoryImpl {
    pool: Pool<Postgres>,
}

impl DebtCategoryRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl DebtCategoryRepository for DebtCategoryRepositoryImpl {
    async fn get_by_name(&self, name: &str) -> HttpResult<Option<DebtCategory>> {
        let row = sqlx::query(r#"SELECT * FROM finance_manager.debt_category WHERE name = $1"#)
            .bind(name.to_uppercase())
            .fetch_optional(&self.pool)
            .await?;

        let result = row.map(|r| DebtCategoryEntity {
            id: r.get("id"),
            name: r.get("name"),
        });

        Ok(result.map(DebtCategory::from))
    }

    async fn insert(&self, category: DebtCategory) -> HttpResult<DebtCategory> {
        let payload = DebtCategoryEntity::from(category);

        let row = sqlx::query(
            r#"INSERT INTO finance_manager.debt_category (
                id,
                name
            )
            VALUES ($1, $2)
            RETURNING *
        "#,
        )
        .bind(payload.id)
        .bind(payload.name)
        .fetch_one(&self.pool)
        .await?;

        let result = DebtCategoryEntity {
            id: row.get("id"),
            name: row.get("name"),
        };

        Ok(DebtCategory::from(result))
    }
    async fn list(&self) -> HttpResult<Vec<DebtCategory>> {
        let rows = sqlx::query(r#"SELECT * FROM finance_manager.debt_category"#)
            .fetch_all(&self.pool)
            .await?;

        let results: Vec<DebtCategoryEntity> = rows
            .into_iter()
            .map(|r| DebtCategoryEntity {
                id: r.get("id"),
                name: r.get("name"),
            })
            .collect();

        Ok(results.into_iter().map(DebtCategory::from).collect())
    }
}

mod entity {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::debt::category::DebtCategory;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct DebtCategoryEntity {
        pub id: Uuid,
        pub name: String,
    }

    impl From<DebtCategory> for DebtCategoryEntity {
        fn from(category: DebtCategory) -> Self {
            DebtCategoryEntity {
                id: *category.id(),
                name: category.name().to_string(),
            }
        }
    }

    impl From<DebtCategoryEntity> for DebtCategory {
        fn from(entity: DebtCategoryEntity) -> Self {
            DebtCategory::from_row(entity.id, entity.name)
        }
    }
}
