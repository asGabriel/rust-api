use axum::{extract::FromRequestParts, http::request::Parts};
use http_error::HttpError;

use crate::modules::{auth::domain::auth_user::AuthUser, routes::AppState};

/// Adding `AuthUser` as a route argument makes the route require a valid bearer token.
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Box<HttpError>;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        state
            .auth_state
            .auth_handler
            .authenticate(&parts.headers)
            .await
    }
}
