use std::sync::Arc;

use async_trait::async_trait;
use axum::http::{header, HeaderMap};
use http_error::{HttpError, HttpResult};
use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::auth::{
    domain::{
        allowed_user::AllowedUser,
        auth_user::AuthUser,
        user::{User, UserResponse},
    },
    repository::{
        allowed_user::DynAllowedUserRepository, google::DynGoogleTokenVerifier,
        user::DynUserRepository,
    },
};

const TOKEN_TTL_HOURS: i64 = 1;

pub type DynAuthHandler = dyn AuthHandler + Send + Sync;

#[async_trait]
pub trait AuthHandler {
    async fn login_with_google(&self, request: GoogleLoginRequest) -> HttpResult<AuthResponse>;
    async fn authenticate(&self, headers: &HeaderMap) -> HttpResult<AuthUser>;
    async fn get_current_user(&self, auth_user: &AuthUser) -> HttpResult<UserResponse>;
}

#[derive(Clone)]
pub struct AuthHandlerImpl {
    pub user_repository: Arc<DynUserRepository>,
    pub allowed_user_repository: Arc<DynAllowedUserRepository>,
    pub google_token_verifier: Arc<DynGoogleTokenVerifier>,
    pub jwt_secret: String,
}

#[async_trait]
impl AuthHandler for AuthHandlerImpl {
    async fn login_with_google(&self, request: GoogleLoginRequest) -> HttpResult<AuthResponse> {
        let identity = self.google_token_verifier.verify(&request.id_token).await?;

        // An existing identity keeps its allowlist entry even if the Google email changes;
        // a new one is only accepted when its email is in the allowlist.
        let (user, allowed_user) = match self
            .user_repository
            .get_by_google_sub(&identity.sub)
            .await?
        {
            Some(mut user) => {
                let allowed_user = self.get_allowed_user(*user.allowed_user_id()).await?;
                user.record_login(&identity);
                (self.user_repository.update(user).await?, allowed_user)
            }
            None => {
                let allowed_user = self
                    .allowed_user_repository
                    .get_by_email(&identity.normalized_email())
                    .await?
                    .ok_or_else(|| {
                        Box::new(HttpError::forbidden("Email is not allowed to sign in"))
                    })?;
                let user = User::new(&allowed_user, &identity);
                (self.user_repository.insert(user).await?, allowed_user)
            }
        };

        Ok(AuthResponse {
            token: self.generate_token(&user)?,
            user: UserResponse::new(&user, &allowed_user),
        })
    }

    async fn authenticate(&self, headers: &HeaderMap) -> HttpResult<AuthUser> {
        let token = Self::extract_bearer_token(headers)?;
        let claims = self.decode_token(token)?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Box::new(HttpError::unauthorized("Invalid token")))?;

        // Loaded on every request so that removing an allowlist entry or changing
        // its role takes effect immediately, without waiting for the token to expire.
        let user = self.get_user(user_id).await?;
        let allowed_user = self.get_allowed_user(*user.allowed_user_id()).await?;

        Ok(AuthUser::new(&user, &allowed_user))
    }

    async fn get_current_user(&self, auth_user: &AuthUser) -> HttpResult<UserResponse> {
        let user = self.get_user(auth_user.user_id()).await?;
        let allowed_user = self.get_allowed_user(*user.allowed_user_id()).await?;

        Ok(UserResponse::new(&user, &allowed_user))
    }
}

impl AuthHandlerImpl {
    async fn get_user(&self, user_id: Uuid) -> HttpResult<User> {
        self.user_repository
            .get_by_id(user_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::unauthorized("User not found")))
    }

    async fn get_allowed_user(&self, allowed_user_id: Uuid) -> HttpResult<AllowedUser> {
        self.allowed_user_repository
            .get_by_id(allowed_user_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::forbidden("Email is not allowed to sign in")))
    }

    fn extract_bearer_token(headers: &HeaderMap) -> HttpResult<&str> {
        headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| Box::new(HttpError::unauthorized("Missing Authorization header")))?
            .strip_prefix("Bearer ")
            .ok_or_else(|| {
                Box::new(HttpError::unauthorized(
                    "Invalid Authorization header format",
                ))
            })
    }

    fn decode_token(&self, token: &str) -> HttpResult<JwtClaims> {
        jsonwebtoken::decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &jsonwebtoken::Validation::default(),
        )
        .map(|data| data.claims)
        .map_err(|_| Box::new(HttpError::unauthorized("Invalid or expired token")))
    }

    fn generate_token(&self, user: &User) -> HttpResult<String> {
        let now = chrono::Utc::now();
        let claims = JwtClaims {
            sub: user.id().to_string(),
            exp: (now + chrono::Duration::hours(TOKEN_TTL_HOURS)).timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )
        .map_err(|_| Box::new(HttpError::internal("Failed to generate token")))
    }
}

pub mod use_cases {
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct GoogleLoginRequest {
        pub id_token: String,
    }
}

pub use use_cases::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}
