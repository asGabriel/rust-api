use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::account::BankAccount, repository::account::dto::BankAccountDto,
};

#[async_trait]
pub trait AccountRepository {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<BankAccount>>;

    // TODO: Add filters
    async fn list(&self) -> HttpResult<Vec<BankAccount>>;

    async fn insert(&self, account: BankAccount) -> HttpResult<BankAccount>;
}

pub type DynAccountRepository = dyn AccountRepository + Send + Sync;
pub struct AccountRepositoryImpl {
    pool: Pool<Postgres>,
}

impl AccountRepositoryImpl {
    pub fn new(pool: &Pool<Postgres>) -> Self {
        Self { pool: pool.clone() }
    }
}

#[async_trait]
impl AccountRepository for AccountRepositoryImpl {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<BankAccount>> {
        let result: Option<BankAccountDto> = sqlx::query_as::<_, BankAccountDto>(
            r#"SELECT * FROM finance_manager.account WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(BankAccount::from))
    }

    async fn list(&self) -> HttpResult<Vec<BankAccount>> {
        let results: Vec<BankAccountDto> = sqlx::query_as::<_, BankAccountDto>(
            r#"SELECT * FROM finance_manager.account ORDER BY created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results.into_iter().map(BankAccount::from).collect())
    }

    async fn insert(&self, account: BankAccount) -> HttpResult<BankAccount> {
        let payload = BankAccountDto::from(account);

        let result = sqlx::query_as::<_, BankAccountDto>(
            r#"
            INSERT INTO finance_manager.account (id, name, owner, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, owner, created_at, updated_at
        "#,
        )
        .bind(payload.id)
        .bind(payload.name)
        .bind(payload.owner)
        .bind(payload.created_at)
        .bind(payload.updated_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(BankAccount::from(result))
    }
}

pub mod dto {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Serialize};
    use sqlx::FromRow;
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
    #[serde(rename_all = "camelCase")]
    pub struct BankAccountDto {
        pub id: Uuid,
        pub name: String,
        pub owner: String,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }
}
