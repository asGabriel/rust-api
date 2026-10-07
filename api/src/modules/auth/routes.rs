use axum::{
    extract::State,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use http_error::HttpResult;

use crate::modules::{
    auth::{domain::auth_user::AuthUser, handler::use_cases::GoogleLoginRequest},
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new()
        .route("/google", post(login_with_google))
        .route("/me", get(get_current_user))
}

async fn login_with_google(
    state: State<AppState>,
    Json(request): Json<GoogleLoginRequest>,
) -> HttpResult<impl IntoResponse> {
    let response = state
        .auth_state
        .auth_handler
        .login_with_google(request)
        .await?;
    Ok(Json(response))
}

async fn get_current_user(
    state: State<AppState>,
    auth_user: AuthUser,
) -> HttpResult<impl IntoResponse> {
    let user = state
        .auth_state
        .auth_handler
        .get_current_user(&auth_user)
        .await?;
    Ok(Json(user))
}
