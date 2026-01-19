use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres, QueryBuilder, Row};
use uuid::Uuid;

use crate::modules::finance_manager::{
    domain::account::BankAccount, handler::account::use_cases::AccountListFilters,
    repository::account::entity::BankAccountEntity,
};

#[async_trait]
pub trait AccountRepository {
    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<BankAccount>>;

    async fn get_by_identification(&self, identification: &str) -> HttpResult<Option<BankAccount>>;

    async fn list(&self, filters: AccountListFilters) -> HttpResult<Vec<BankAccount>>;

    async fn insert(&self, account: BankAccount) -> HttpResult<BankAccount>;

    async fn update(&self, account: BankAccount) -> HttpResult<()>;
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
    async fn update(&self, account: BankAccount) -> HttpResult<()> {
        let payload = BankAccountEntity::from(account);

        sqlx::query(
            r#"
            UPDATE finance_manager.account SET 
                name = $2,
                owner = $3,
                configuration = $4,
                updated_at = $5
            WHERE id = $1"#,
        )
        .bind(payload.id)
        .bind(payload.name)
        .bind(payload.owner)
        .bind(serde_json::to_value(payload.configuration).unwrap())
        .bind(payload.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn get_by_identification(&self, identification: &str) -> HttpResult<Option<BankAccount>> {
        let identification_num: i32 = identification.parse().map_err(|_| {
            http_error::HttpError::bad_request(format!(
                "Invalid identification format: {}",
                identification
            ))
        })?;

        let row = sqlx::query(r#"SELECT * FROM finance_manager.account WHERE identification = $1"#)
            .bind(identification_num)
            .fetch_optional(&self.pool)
            .await?;

        let result = row.map(|r| BankAccountEntity {
            id: r.get("id"),
            client_id: r.get("client_id"),
            name: r.get("name"),
            owner: r.get("owner"),
            identification: r.get::<i32, _>("identification").to_string(),
            configuration: serde_json::from_value(r.get("configuration")).unwrap(),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        });

        Ok(result.map(BankAccount::from))
    }

    async fn get_by_id(&self, id: Uuid) -> HttpResult<Option<BankAccount>> {
        let row = sqlx::query(r#"SELECT * FROM finance_manager.account WHERE id = $1"#)
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        let result = row.map(|r| BankAccountEntity {
            id: r.get("id"),
            client_id: r.get("client_id"),
            name: r.get("name"),
            owner: r.get("owner"),
            identification: r.get::<i32, _>("identification").to_string(),
            configuration: serde_json::from_value(r.get("configuration")).unwrap(),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        });

        Ok(result.map(BankAccount::from))
    }

    async fn list(&self, filters: AccountListFilters) -> HttpResult<Vec<BankAccount>> {
        let mut builder = QueryBuilder::new("SELECT * FROM finance_manager.account WHERE 1=1");

        if let Some(client_id) = filters.client_id {
            builder.push(" AND client_id = ");
            builder.push_bind(client_id);
        }

        if let Some(ids) = filters.ids {
            builder.push(" AND id = ANY(");
            builder.push_bind(ids);
            builder.push(")");
        }

        if let Some(identifications) = filters.identifications {
            let identifications: Vec<i32> = identifications
                .iter()
                .map(|i| i.parse::<i32>().unwrap())
                .collect();
            builder.push(" AND identification = ANY(");
            builder.push_bind(identifications);
            builder.push(")");
        }

        let query = builder.build();
        let rows = query.fetch_all(&self.pool).await?;

        let rows: Vec<BankAccountEntity> = rows
            .into_iter()
            .map(|r| BankAccountEntity {
                id: r.get("id"),
                client_id: r.get("client_id"),
                name: r.get("name"),
                owner: r.get("owner"),
                identification: r.get::<i32, _>("identification").to_string(),
                configuration: serde_json::from_value(r.get("configuration")).unwrap(),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            })
            .collect();

        Ok(rows.into_iter().map(BankAccount::from).collect())
    }

    async fn insert(&self, account: BankAccount) -> HttpResult<BankAccount> {
        let payload = BankAccountEntity::from(account);

        let row = sqlx::query(
            r#"
            INSERT INTO finance_manager.account (id, client_id, name, owner, configuration, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING *
        "#,
        )
        .bind(payload.id)
        .bind(payload.client_id)
        .bind(payload.name)
        .bind(payload.owner)
        .bind(serde_json::to_value(payload.configuration).unwrap())
        .bind(payload.created_at)
        .bind(payload.updated_at)
        .fetch_one(&self.pool)
        .await?;

        let result = BankAccountEntity {
            id: row.get("id"),
            client_id: row.get("client_id"),
            name: row.get("name"),
            owner: row.get("owner"),
            identification: row.get::<i32, _>("identification").to_string(),
            configuration: serde_json::from_value(row.get("configuration")).unwrap(),
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

    use crate::modules::finance_manager::domain::account::{
        configuration::AccountConfiguration, BankAccount,
    };

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BankAccountEntity {
        pub id: Uuid,
        pub client_id: Uuid,
        pub name: String,
        pub owner: String,
        pub identification: String,
        pub configuration: AccountConfiguration,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }

    impl From<BankAccount> for BankAccountEntity {
        fn from(bank_account: BankAccount) -> Self {
            BankAccountEntity {
                id: *bank_account.id(),
                client_id: *bank_account.client_id(),
                name: bank_account.name().to_string(),
                owner: bank_account.owner().to_string(),
                identification: bank_account.identification().to_string(),
                configuration: bank_account.configuration().clone(),
                created_at: bank_account.created_at().naive_utc(),
                updated_at: bank_account.updated_at().map(|dt| dt.naive_utc()),
            }
        }
    }

    impl From<BankAccountEntity> for BankAccount {
        fn from(dto: BankAccountEntity) -> Self {
            BankAccount::from_row(
                dto.id,
                dto.client_id,
                dto.name,
                dto.owner,
                dto.identification,
                dto.configuration,
                dto.created_at.and_utc(),
                dto.updated_at.map(|dt| dt.and_utc()),
            )
        }
    }
}
