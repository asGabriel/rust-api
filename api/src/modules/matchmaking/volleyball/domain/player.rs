use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_at: Option<DateTime<Utc>>,
}

impl Player {
    pub fn new(name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            created_at: Utc::now(),
            updated_at: None,
            deleted_at: None,
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.updated_at = Some(Utc::now());
    }

    pub fn soft_delete(&mut self) {
        self.deleted_at = Some(Utc::now());
        self.updated_at = Some(Utc::now());
    }

    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

getters!(
    Player {
        id: Uuid,
        name: String,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        deleted_at: Option<DateTime<Utc>>,
    }
);

from_row_constructor! {
    Player {
        id: Uuid,
        name: String,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        deleted_at: Option<DateTime<Utc>>,
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerFilters {
    ids: Option<Vec<Uuid>>,
    name: Option<String>,
}

impl PlayerFilters {
    pub fn with_ids(mut self, ids: Vec<Uuid>) -> Self {
        self.ids = Some(ids);
        self
    }

    pub fn with_optional_ids(mut self, ids: Option<Vec<Uuid>>) -> Self {
        if let Some(ids) = ids {
            self.ids = Some(ids);
        }
        self
    }

    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    pub fn with_optional_name(mut self, name: Option<String>) -> Self {
        if let Some(name) = name {
            self.name = Some(name);
        }
        self
    }
}

getters!(
    PlayerFilters {
        ids: Option<Vec<Uuid>>,
        name: Option<String>,
    }
);
