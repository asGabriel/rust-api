use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, Row};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::account::BankAccount, repository::account::entity::BankAccountEntity,
};

#[async_trait]
pub trait AccountRepository {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<BankAccount>>;

    async fn get_by_identification(&self, identification: &str) -> HttpResult<Option<BankAccount>>;

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
    async fn get_by_identification(&self, identification: &str) -> HttpResult<Option<BankAccount>> {
        let identification_num: i32 = identification.parse().map_err(|_| {
            http_error::HttpError::bad_request(format!(
                "Invalid identification format: {}",
                identification
            ))
        })?;

        let row = sqlx::query(
            r#"SELECT id, name, owner, identification, created_at, updated_at FROM finance_manager.account WHERE identification = $1"#
        )
        .bind(identification_num)
        .fetch_optional(&self.pool)
        .await?;

        let result = row.map(|r| BankAccountEntity {
            id: r.get("id"),
            name: r.get("name"),
            owner: r.get("owner"),
            identification: r.get::<i32, _>("identification").to_string(),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        });

        Ok(result.map(BankAccount::from))
    }

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<BankAccount>> {
        let row = sqlx::query(
            r#"SELECT id, name, owner, identification, created_at, updated_at FROM finance_manager.account WHERE id = $1"#
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let result = row.map(|r| BankAccountEntity {
            id: r.get("id"),
            name: r.get("name"),
            owner: r.get("owner"),
            identification: r.get::<i32, _>("identification").to_string(),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        });

        Ok(result.map(BankAccount::from))
    }

    async fn list(&self) -> HttpResult<Vec<BankAccount>> {
        let rows = sqlx::query(
            r#"SELECT id, name, owner, identification, created_at, updated_at FROM finance_manager.account ORDER BY created_at DESC"#
        )
        .fetch_all(&self.pool)
        .await?;

        let results: Vec<BankAccountEntity> = rows
            .into_iter()
            .map(|r| BankAccountEntity {
                id: r.get("id"),
                name: r.get("name"),
                owner: r.get("owner"),
                identification: r.get::<i32, _>("identification").to_string(),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect();

        Ok(results.into_iter().map(BankAccount::from).collect())
    }

    async fn insert(&self, account: BankAccount) -> HttpResult<BankAccount> {
        let payload = BankAccountEntity::from(account);

        let row = sqlx::query(
            r#"
            INSERT INTO finance_manager.account (id, name, owner, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, owner, identification, created_at, updated_at
        "#,
        )
        .bind(payload.id)
        .bind(payload.name)
        .bind(payload.owner)
        .bind(payload.created_at)
        .bind(payload.updated_at)
        .fetch_one(&self.pool)
        .await?;

        let result = BankAccountEntity {
            id: row.get("id"),
            name: row.get("name"),
            owner: row.get("owner"),
            identification: row.get::<i32, _>("identification").to_string(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        };

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
