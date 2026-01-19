use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    id: Uuid,
    username: String,
    email: String,
    #[serde(skip_serializing)]
    password_hash: String,
    name: String,
    is_active: bool,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl User {
    pub fn new(username: String, email: String, password_hash: String, name: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            username,
            email,
            password_hash,
            name,
            is_active: true,
            created_at: Utc::now(),
            updated_at: None,
        }
    }

    pub fn verify_password(&self, password: &str) -> bool {
        bcrypt::verify(password, &self.password_hash).unwrap_or(false)
    }

    pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
        bcrypt::hash(password, bcrypt::DEFAULT_COST)
    }
}

getters! {
    User {
        id: Uuid,
        username: String,
        email: String,
        password_hash: String,
        name: String,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

from_row_constructor! {
    User {
        id: Uuid,
        username: String,
        email: String,
        password_hash: String,
        name: String,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub name: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: *user.id(),
            username: user.username().clone(),
            email: user.email().clone(),
            name: user.name().clone(),
            is_active: *user.is_active(),
            created_at: *user.created_at(),
            updated_at: *user.updated_at(),
        }
    }
}
