use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use util::{from_row_constructor, getters};
use uuid::Uuid;

pub mod configuration;

use crate::modules::{
    chat_bot::domain::formatter::ChatFormatter,
    finance_manager::{
        domain::account::configuration::AccountConfiguration,
        handler::account::use_cases::UpdateAccountRequest,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BankAccount {
    id: Uuid,
    client_id: Uuid,
    name: String,
    owner: String,
    identification: String,
    configuration: AccountConfiguration,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl BankAccount {
    pub fn new(client_id: Uuid, name: String, owner: String, configuration: AccountConfiguration) -> Self {
        let uuid = Uuid::new_v4();

        Self {
            id: uuid,
            client_id,
            name,
            owner,
            identification: String::new(),
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


getters! {
    BankAccount {
        id: Uuid,
        client_id: Uuid,
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
        client_id: Uuid,
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
        if items.is_empty() {
            return "📋 Nenhuma conta cadastrada".to_string();
        }

        // Agrupar contas por owner
        let mut accounts_by_owner: BTreeMap<String, Vec<&Self>> = BTreeMap::new();
        for account in items.iter() {
            accounts_by_owner
                .entry(account.owner().clone())
                .or_default()
                .push(account);
        }

        let mut output = format!("📋 Contas cadastradas ({})", items.len());

        // Listar contas agrupadas por owner
        for (owner, accounts) in accounts_by_owner.iter() {
            output.push_str(&format!("\n\n👤 {} ({})", owner, accounts.len()));
            for account in accounts.iter() {
                output.push_str(&format!(
                    "\n {} - {}",
                    account.identification(),
                    account.name()
                ));
            }
        }

        output
    }
}
