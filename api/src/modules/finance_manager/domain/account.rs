use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::finance_manager::repository::account::dto::BankAccountDto;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankAccount {
    /// Unique identifier
    id: Uuid,
    /// The name of the bank account
    name: String,
    /// The owner of the bank account
    owner: String,
    /// The date of the creation of the bank account
    created_at: DateTime<Utc>,
    /// The date of the last update of the bank account
    updated_at: Option<DateTime<Utc>>,
}

impl From<BankAccount> for BankAccountDto {
    fn from(bank_account: BankAccount) -> Self {
        BankAccountDto {
            id: bank_account.id,
            name: bank_account.name,
            owner: bank_account.owner,
            created_at: bank_account.created_at.naive_utc(),
            updated_at: bank_account.updated_at.map(|dt| dt.naive_utc()),
        }
    }
}

impl From<BankAccountDto> for BankAccount {
    fn from(dto: BankAccountDto) -> Self {
        BankAccount {
            id: dto.id,
            name: dto.name,
            owner: dto.owner,
            created_at: dto.created_at.and_utc(),
            updated_at: dto.updated_at.map(|dt| dt.and_utc()),
        }
    }
}
