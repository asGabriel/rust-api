use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    id: Uuid,
    client_id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl List {
    pub fn new(client_id: Uuid, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            client_id,
            name,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.updated_at = Some(Utc::now());
    }
}

getters!(
    List {
        id: Uuid,
        client_id: Uuid,
        name: String,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
);

from_row_constructor! {
    List {
        id: Uuid,
        client_id: Uuid,
        name: String,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
