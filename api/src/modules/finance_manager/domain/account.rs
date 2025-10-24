use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

use crate::utils::generate_random_identification;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankAccount {
    /// Unique identifier
    id: Uuid,
    /// The name of the bank account
    name: String,
    /// The owner of the bank account
    owner: String,
    /// The identification of the account; It's a human readable identifier for the account.
    identification: String,
    /// The date of the creation of the bank account
    created_at: DateTime<Utc>,
    /// The date of the last update of the bank account
    updated_at: Option<DateTime<Utc>>,
}

impl BankAccount {
    pub fn new(name: String, owner: String) -> Self {
        let uuid = Uuid::new_v4();
        let identification = generate_random_identification(uuid);

        Self {
            id: uuid,
            name,
            owner,
            identification,
            created_at: Utc::now(),
            updated_at: None,
        }
    }
}

getters! {
    BankAccount {
        id: Uuid,
        name: String,
        owner: String,
        identification: String,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

from_row_constructor! {
    BankAccount {
        id: Uuid,
        name: String,
        owner: String,
        identification: String,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
