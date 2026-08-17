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

impl From<String> for Gender {
    fn from(s: String) -> Self {
        match s.as_str() {
            "FEMALE" => Gender::Female,
            _ => Gender::Male,
        }
    }
}

impl From<Gender> for String {
    fn from(gender: Gender) -> Self {
        match gender {
            Gender::Male => "MALE".to_string(),
            Gender::Female => "FEMALE".to_string(),
        }
    }
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

impl From<&sqlx::postgres::PgRow> for Player {
    fn from(row: &sqlx::postgres::PgRow) -> Self {
        use sqlx::Row;

        Self {
            id: row.get("id"),
            name: row.get("name"),
            gender: row.get::<String, _>("gender").into(),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
