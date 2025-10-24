use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::account::BankAccount, repository::account::entity::BankAccountEntity,
};

#[async_trait]
pub trait AccountRepository {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<BankAccount>>;

    async fn get_by_identification(
        &self,
        identification: String,
    ) -> HttpResult<Option<BankAccount>>;

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
    async fn get_by_identification(
        &self,
        identification: String,
    ) -> HttpResult<Option<BankAccount>> {
        let result: Option<BankAccountEntity> = sqlx::query_as!(
            BankAccountEntity,
            r#"SELECT * FROM finance_manager.account WHERE identification = $1"#,
            identification,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(BankAccount::from))
    }

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<BankAccount>> {
        let result: Option<BankAccountEntity> = sqlx::query_as!(
            BankAccountEntity,
            r#"SELECT * FROM finance_manager.account WHERE id = $1"#,
            id,
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.map(BankAccount::from))
    }

    async fn list(&self) -> HttpResult<Vec<BankAccount>> {
        let results = sqlx::query_as!(
            BankAccountEntity,
            r#"SELECT * FROM finance_manager.account ORDER BY created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(results.into_iter().map(BankAccount::from).collect())
    }

    async fn insert(&self, account: BankAccount) -> HttpResult<BankAccount> {
        let payload = BankAccountEntity::from(account);

        let result = sqlx::query_as!(
            BankAccountEntity,
            r#"
            INSERT INTO finance_manager.account (id, name, owner, identification, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, name, owner, identification, created_at, updated_at
        "#,
            payload.id,
            payload.name,
            payload.owner,
            payload.identification,
            payload.created_at,
            payload.updated_at,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(BankAccount::from(result))
    }
}

pub mod entity {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::account::BankAccount;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BankAccountEntity {
        pub id: Uuid,
        pub name: String,
        pub owner: String,
        pub identification: String,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<BankAccount> for BankAccountEntity {
        fn from(bank_account: BankAccount) -> Self {
            BankAccountEntity {
                id: *bank_account.id(),
                name: bank_account.name().to_string(),
                owner: bank_account.owner().to_string(),
                identification: bank_account.identification().to_string(),
                created_at: bank_account.created_at().naive_utc(),
                updated_at: bank_account.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<BankAccountEntity> for BankAccount {
        fn from(dto: BankAccountEntity) -> Self {
            BankAccount::from_row(
                dto.id,
                dto.name,
                dto.owner,
                dto.identification,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
