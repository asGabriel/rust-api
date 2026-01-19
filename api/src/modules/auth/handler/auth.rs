use std::sync::Arc;

use async_trait::async_trait;
use axum::http::{header, HeaderMap};
use http_error::{HttpError, HttpResult};
use jsonwebtoken::{encode, DecodingKey, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::modules::auth::{
    domain::user::{User, UserResponse},
    repository::user::DynUserRepository,
};

pub type DynAuthHandler = dyn AuthHandler + Send + Sync;

#[async_trait]
pub trait AuthHandler {
    async fn register(&self, request: RegisterRequest) -> HttpResult<AuthResponse>;
    async fn login(&self, request: LoginRequest) -> HttpResult<AuthResponse>;
    async fn get_user_by_id(&self, id: Uuid) -> HttpResult<Option<User>>;
    async fn authenticate(&self, headers: &HeaderMap) -> HttpResult<User>;
    fn decode_token(&self, token: &str) -> HttpResult<JwtClaims>;
    fn extract_token_from_header(&self, headers: &HeaderMap) -> HttpResult<String>;
}

#[derive(Clone)]
pub struct AuthHandlerImpl {
    pub user_repository: Arc<DynUserRepository>,
    pub jwt_secret: String,
}

#[async_trait]
impl AuthHandler for AuthHandlerImpl {
    async fn register(&self, request: RegisterRequest) -> HttpResult<AuthResponse> {
        if self
            .user_repository
            .get_by_username(&request.username)
            .await?
            .is_some()
        {
            return Err(Box::new(HttpError::conflict("Username already exists")));
        }

        if self
            .user_repository
            .get_by_email(&request.email)
            .await?
            .is_some()
        {
            return Err(Box::new(HttpError::conflict("Email already registered")));
        }

        let password_hash = User::hash_password(&request.password)
            .map_err(|_| Box::new(HttpError::internal("Failed to hash password")))?;

        let user = User::new(
            request.client_id,
            request.username,
            request.email,
            password_hash,
            request.name,
        );
        let user = self.user_repository.insert(user).await?;
        let token = self.generate_token(&user)?;

        Ok(AuthResponse {
            token,
            user: user.into(),
        })
    }

    async fn login(&self, request: LoginRequest) -> HttpResult<AuthResponse> {
        let user = self
            .user_repository
            .get_by_username(&request.username)
            .await?
            .ok_or_else(|| Box::new(HttpError::unauthorized("Invalid username or password")))?;

        if !user.is_active() {
            return Err(Box::new(HttpError::unauthorized(
                "User account is deactivated",
            )));
        }

        if !user.verify_password(&request.password) {
            return Err(Box::new(HttpError::unauthorized(
                "Invalid username or password",
            )));
        }

        let token = self.generate_token(&user)?;

        Ok(AuthResponse {
            token,
            user: user.into(),
        })
    }

    async fn get_user_by_id(&self, id: Uuid) -> HttpResult<Option<User>> {
        self.user_repository.get_by_id(id).await
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

    fn extract_token_from_header(&self, headers: &HeaderMap) -> HttpResult<String> {
        let auth_header = headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or_else(|| Box::new(HttpError::unauthorized("Missing Authorization header")))?;

        if !auth_header.starts_with("Bearer ") {
            return Err(Box::new(HttpError::unauthorized(
                "Invalid Authorization header format",
            )));
        }

        Ok(auth_header[7..].to_string())
    }

    async fn authenticate(&self, headers: &HeaderMap) -> HttpResult<User> {
        let token = self.extract_token_from_header(headers)?;
        let claims = self.decode_token(&token)?;

        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| Box::new(HttpError::unauthorized("Invalid token")))?;

        let user = self
            .get_user_by_id(user_id)
            .await?
            .ok_or_else(|| Box::new(HttpError::unauthorized("User not found")))?;

        if !user.is_active() {
            return Err(Box::new(HttpError::unauthorized(
                "User account is deactivated",
            )));
        }

        Ok(user)
    }
}

impl AuthHandlerImpl {
    fn generate_token(&self, user: &User) -> HttpResult<String> {
        let claims = JwtClaims {
            sub: user.id().to_string(),
            client_id: user.client_id().to_string(),
            username: user.username().clone(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: chrono::Utc::now().timestamp() as usize,
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
    use uuid::Uuid;

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct RegisterRequest {
        pub client_id: Uuid,
        pub username: String,
        pub email: String,
        pub password: String,
        pub name: String,
    }

    #[derive(Debug, Clone, Deserialize, Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct LoginRequest {
        pub username: String,
        pub password: String,
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
    pub client_id: String,
    pub username: String,
    pub exp: usize,
    pub iat: usize,
}
