use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    Admin,
    Member,
}

impl From<String> for Role {
    fn from(s: String) -> Self {
        match s.as_str() {
            "ADMIN" => Role::Admin,
            _ => Role::Member,
        }
    }
}

impl From<Role> for String {
    fn from(role: Role) -> Self {
        match role {
            Role::Admin => "ADMIN".to_string(),
            Role::Member => "MEMBER".to_string(),
        }
    }
}

/// An email authorized to sign in, bound to the tenant whose data it accesses.
/// Removing it revokes access, since the linked `User` is deleted with it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowedUser {
    id: Uuid,
    email: String,
    tenant_id: Uuid,
    role: Role,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
}

impl AllowedUser {
    pub fn new(email: &str, tenant_id: Uuid, role: Role) -> Self {
        Self {
            id: Uuid::new_v4(),
            email: email.trim().to_lowercase(),
            tenant_id,
            role,
            created_at: Utc::now(),
            updated_at: None,
        }
    }
}

getters! {
    AllowedUser {
        id: Uuid,
        email: String,
        tenant_id: Uuid,
        role: Role,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}

from_row_constructor! {
    AllowedUser {
        id: Uuid,
        email: String,
        tenant_id: Uuid,
        role: Role,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
    }
}
