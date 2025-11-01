use std::sync::Arc;

use async_trait::async_trait;
use http_error::{ext::OptionHttpExt, HttpResult};

use crate::modules::finance_manager::{
    domain::account::BankAccount,
    handler::account::use_cases::{AccountListFilters, CreateAccountRequest, UpdateAccountRequest},
    repository::account::DynAccountRepository,
};

pub type DynAccountHandler = dyn AccountHandler + Send + Sync;

#[async_trait]
pub trait AccountHandler {
    async fn create_account(&self, request: CreateAccountRequest) -> HttpResult<BankAccount>;

    async fn list_accounts(&self, filters: AccountListFilters) -> HttpResult<Vec<BankAccount>>;

    async fn update_account(&self, request: UpdateAccountRequest) -> HttpResult<BankAccount>;
}

#[derive(Clone)]
pub struct AccountHandlerImpl {
    pub account_repository: Arc<DynAccountRepository>,
}

#[async_trait]
impl AccountHandler for AccountHandlerImpl {
    async fn update_account(&self, request: UpdateAccountRequest) -> HttpResult<BankAccount> {
        let mut account = self
            .account_repository
            .get_by_identification(&request.identification)
            .await?
            .or_not_found("account", &request.identification)?;

        account.update(&request);
        // Safe unwrap because we already checked if the account exists
        self.account_repository.update(account.clone()).await?;

        Ok(account)
    }

    async fn create_account(&self, request: CreateAccountRequest) -> HttpResult<BankAccount> {
        let bank_account = BankAccount::from(request);

        self.account_repository.insert(bank_account).await
    }

    async fn list_accounts(&self, filters: AccountListFilters) -> HttpResult<Vec<BankAccount>> {
        self.account_repository.list(filters).await
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use crate::modules::finance_manager::domain::account::configuration::AccountConfiguration;

    #[derive(Debug, Clone, Default, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct AccountListFilters {
        pub ids: Option<Vec<Uuid>>,
        pub identifications: Option<Vec<String>>,
    }

    impl AccountListFilters {
        pub fn new() -> Self {
            Self {
                ..Default::default()
            }
        }

        pub fn with_ids(mut self, ids: Vec<Uuid>) -> Self {
            self.ids = Some(ids);
            self
        }

        pub fn with_identifications(mut self, identifications: Vec<String>) -> Self {
            self.identifications = Some(identifications);
            self
        }
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateAccountRequest {
        pub name: String,
        pub owner: String,
        pub configuration: Option<AccountConfiguration>,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateAccountRequest {
        pub identification: String,
        pub name: Option<String>,
        pub owner: Option<String>,
        pub configuration: Option<AccountConfiguration>,
    }
}
