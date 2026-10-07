use uuid::Uuid;

use crate::modules::auth::domain::allowed_user::{AllowedUser, Role};
use crate::modules::auth::domain::user::User;

/// The authenticated caller of a request, resolved from its bearer token.
#[derive(Debug, Clone)]
pub struct AuthUser {
    user_id: Uuid,
    tenant_id: Uuid,
    role: Role,
}

impl AuthUser {
    pub fn new(user: &User, allowed_user: &AllowedUser) -> Self {
        Self {
            user_id: *user.id(),
            tenant_id: *allowed_user.tenant_id(),
            role: *allowed_user.role(),
        }
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn tenant_id(&self) -> Uuid {
        self.tenant_id
    }

    pub fn role(&self) -> Role {
        self.role
    }

    pub fn is_admin(&self) -> bool {
        self.role == Role::Admin
    }
}
