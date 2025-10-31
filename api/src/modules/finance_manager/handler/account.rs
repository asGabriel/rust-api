use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;

use crate::modules::finance_manager::{
    domain::account::BankAccount, handler::account::use_cases::CreateAccountRequest,
    repository::account::DynAccountRepository,
};

pub type DynAccountHandler = dyn AccountHandler + Send + Sync;

#[async_trait]
pub trait AccountHandler {
    async fn create_account(&self, request: CreateAccountRequest) -> HttpResult<BankAccount>;

    async fn list_accounts(&self) -> HttpResult<Vec<BankAccount>>;
}

#[derive(Clone)]
pub struct AccountHandlerImpl {
    pub account_repository: Arc<DynAccountRepository>,
}

#[async_trait]
impl AccountHandler for AccountHandlerImpl {
    async fn create_account(&self, request: CreateAccountRequest) -> HttpResult<BankAccount> {
        let bank_account = BankAccount::from(request);

        self.account_repository.insert(bank_account).await
    }

    async fn list_accounts(&self) -> HttpResult<Vec<BankAccount>> {
        self.account_repository.list().await
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};

    use crate::modules::finance_manager::domain::account::configuration::AccountConfiguration;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateAccountRequest {
        pub name: String,
        pub owner: String,
        pub configuration: Option<AccountConfiguration>,
    }
}
