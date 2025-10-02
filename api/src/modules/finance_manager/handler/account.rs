use std::sync::Arc;

use async_trait::async_trait;
use http_error::HttpResult;
use serde::{Deserialize, Serialize};

use crate::modules::finance_manager::{domain::account::{BankAccount}, repository::account::DynAccountRepository};

pub type DynAccountHandler = dyn AccountHandler + Send + Sync;

#[async_trait]
pub trait AccountHandler {
    async fn create_account(&self, request: CreateAccountRequest) -> HttpResult<BankAccount>;
}

pub struct AccountHandlerImpl {
    pub account_repository: Arc<DynAccountRepository>,
}

#[async_trait]
impl AccountHandler for AccountHandlerImpl {
    async fn create_account(&self, request: CreateAccountRequest) -> HttpResult<BankAccount> {
        let bank_account = BankAccount::new(request.name, request.owner);
        self.account_repository.insert(bank_account).await
    }
}


// Use cases

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateAccountRequest {
    name: String,
    owner: String,
}
