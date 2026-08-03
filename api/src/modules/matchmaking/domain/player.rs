use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::getters;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Gender {
    Male,
    Female,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Player {
    id: Uuid,
    name: String,
    gender: Gender,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl Player {
    pub fn new(name: String, gender: Gender) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            gender,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
        self.updated_at = Some(Utc::now());
    }

    pub fn set_gender(&mut self, gender: Gender) {
        self.gender = gender;
        self.updated_at = Some(Utc::now());
    }
}

getters! {
    Player {
        id: Uuid,
        name: String,
        gender: Gender,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
