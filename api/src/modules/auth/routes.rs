use axum::{
    extract::State,
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use http_error::HttpResult;

use crate::modules::{
    auth::{
        domain::user::UserResponse,
        handler::use_cases::{LoginRequest, RegisterRequest},
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/me", get(get_current_user))
}

async fn register(
    state: State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> HttpResult<impl IntoResponse> {
    let response = state.auth_state.auth_handler.register(request).await?;
    Ok(Json(response))
}

async fn login(
    state: State<AppState>,
    Json(request): Json<LoginRequest>,
) -> HttpResult<impl IntoResponse> {
    let response = state.auth_state.auth_handler.login(request).await?;
    Ok(Json(response))
}

async fn get_current_user(
    state: State<AppState>,
    headers: HeaderMap,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;
    Ok(Json(UserResponse::from(user)))
}
