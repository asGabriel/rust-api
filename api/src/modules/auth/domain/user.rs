use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use util::{from_row_constructor, getters};
use uuid::Uuid;

use crate::modules::auth::domain::{
    allowed_user::{AllowedUser, Role},
    google::GoogleIdentity,
};

/// A Google identity linked to an `AllowedUser`, created on its first sign-in.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    id: Uuid,
    allowed_user_id: Uuid,
    google_sub: String,
    email: String,
    name: String,
    avatar_url: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: Option<DateTime<Utc>>,
    last_login_at: DateTime<Utc>,
}

impl User {
    pub fn new(allowed_user: &AllowedUser, identity: &GoogleIdentity) -> Self {
        let now = Utc::now();

        Self {
            id: Uuid::new_v4(),
            allowed_user_id: *allowed_user.id(),
            google_sub: identity.sub.clone(),
            email: identity.normalized_email(),
            name: Self::display_name(identity),
            avatar_url: identity.picture.clone(),
            created_at: now,
            updated_at: None,
            last_login_at: now,
        }
    }

    /// Refreshes the profile with the latest data Google returned on sign-in.
    pub fn record_login(&mut self, identity: &GoogleIdentity) {
        let now = Utc::now();

        self.email = identity.normalized_email();
        self.name = Self::display_name(identity);
        self.avatar_url = identity.picture.clone();
        self.updated_at = Some(now);
        self.last_login_at = now;
    }

    fn display_name(identity: &GoogleIdentity) -> String {
        identity
            .name
            .clone()
            .unwrap_or_else(|| identity.normalized_email())
    }
}

getters! {
    User {
        id: Uuid,
        allowed_user_id: Uuid,
        google_sub: String,
        email: String,
        name: String,
        avatar_url: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        last_login_at: DateTime<Utc>,
    }
}

from_row_constructor! {
    User {
        id: Uuid,
        allowed_user_id: Uuid,
        google_sub: String,
        email: String,
        name: String,
        avatar_url: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: Option<DateTime<Utc>>,
        last_login_at: DateTime<Utc>,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub name: String,
    pub avatar_url: Option<String>,
    pub tenant_id: Uuid,
    pub role: Role,
    pub last_login_at: DateTime<Utc>,
}

impl UserResponse {
    pub fn new(user: &User, allowed_user: &AllowedUser) -> Self {
        Self {
            id: *user.id(),
            email: user.email().clone(),
            name: user.name().clone(),
            avatar_url: user.avatar_url().clone(),
            tenant_id: *allowed_user.tenant_id(),
            role: *allowed_user.role(),
            last_login_at: *user.last_login_at(),
        }
    }
}
