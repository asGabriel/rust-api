use async_trait::async_trait;
use http_error::HttpResult;
use sqlx::{Pool, Postgres};

use crate::modules::finance_manager::{
    domain::account::BankAccount, repository::account::dto::BankAccountDto,
};

#[async_trait]
pub trait FinancialInstrumentRepository {
    async fn create(
        &self,
        pool: &Pool<Postgres>,
        financial_instrument: BankAccount,
    ) -> HttpResult<BankAccount>;
}

pub struct FinancialInstrumentRepositoryImpl;

#[async_trait]
impl FinancialInstrumentRepository for FinancialInstrumentRepositoryImpl {
    async fn create(
        &self,
        pool: &Pool<Postgres>,
        financial_instrument: BankAccount,
    ) -> HttpResult<BankAccount> {
        let payload = BankAccountDto::from(financial_instrument);

        let result = sqlx::query!(
            r#"
            INSERT INTO finance_manager.account (id, name, owner, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, name, owner, created_at, updated_at
        "#,
            payload.id,
            payload.name,
            payload.owner,
            payload.created_at,
            payload.updated_at,
        )
        .fetch_one(pool)
        .await?;

        let dto = BankAccountDto {
            id: result.id,
            name: result.name,
            owner: result.owner,
            created_at: result.created_at,
            updated_at: result.updated_at,
        };

        Ok(BankAccount::from(dto))
    }
}

pub mod dto {
    use chrono::NaiveDateTime;
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct BankAccountDto {
        pub id: Uuid,
        pub name: String,
        pub owner: String,
        pub created_at: NaiveDateTime,
        pub updated_at: Option<NaiveDateTime>,
    }
}
