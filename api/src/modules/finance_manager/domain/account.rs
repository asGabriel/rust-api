use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

pub mod configuration;

use crate::modules::{
    chat_bot::domain::formatter::ChatFormatter,
    finance_manager::{
        domain::account::configuration::AccountConfiguration,
        handler::account::use_cases::{CreateAccountRequest, UpdateAccountRequest},
    },
};

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
    /// The configuration of the bank account
    configuration: AccountConfiguration,

    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl BankAccount {
    pub fn new(name: String, owner: String, configuration: AccountConfiguration) -> Self {
        let uuid = Uuid::new_v4();

        Self {
            id: uuid,
            name,
            owner,
            identification: String::new(), // Will be set by database autoincrement
            configuration,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    /// Returns the default due date for the account configuration.
    pub fn default_due_date(&self) -> Option<NaiveDate> {
        self.configuration.default_due_date()
    }

    pub fn update(&mut self, request: &UpdateAccountRequest) {
        if let Some(name) = &request.name {
            self.name = name.clone();
        }
        if let Some(owner) = &request.owner {
            self.owner = owner.clone();
        }
        if let Some(configuration) = &request.configuration {
            self.configuration = configuration.clone();
        }

        self.updated_at = Some(Utc::now());
    }
}

impl From<CreateAccountRequest> for BankAccount {
    fn from(request: CreateAccountRequest) -> Self {
        let configuration = request
            .configuration
            .unwrap_or(AccountConfiguration::default());
        BankAccount::new(request.name, request.owner, configuration)
    }
}

getters! {
    BankAccount {
        id: Uuid,
        name: String,
        owner: String,
        identification: String,
        configuration: AccountConfiguration,
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
        configuration: AccountConfiguration,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

impl ChatFormatter for BankAccount {
    fn format_for_chat(&self) -> String {
        format!(
            "🏦 Conta: {}\n🆔 ID: {}\n👤 Dono: {}",
            self.name(),
            self.identification(),
            self.owner()
        )
    }

    fn format_list_for_chat(items: &[Self]) -> String {
        let mut output = format!("📋 Contas cadastradas ({})", items.len());

        for account in items.iter() {
            output.push_str(&format!(
                "\n🆔 {} - {}",
                account.identification(),
                account.name()
            ));
        }

        output
    }
}
