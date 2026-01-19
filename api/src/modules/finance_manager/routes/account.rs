use axum::{
    extract::State,
    http::HeaderMap,
    response::IntoResponse,
    routing::{patch, post},
    Json, Router,
};
use http_error::HttpResult;

use crate::modules::{
    finance_manager::handler::account::use_cases::{
        AccountListFilters, CreateAccountRequest, UpdateAccountRequest,
    },
    routes::AppState,
};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest(
        "/account",
        Router::new()
            .route("/", post(create_account))
            .route("/list", post(list_accounts))
            .route("/", patch(update_account)),
    )
}

async fn update_account(
    state: State<AppState>,
    headers: HeaderMap,
    Json(request): Json<UpdateAccountRequest>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;
    let account = state
        .finance_manager_state
        .account_handler
        .update_account(*user.client_id(), request)
        .await?;

    Ok(Json(account))
}

async fn create_account(
    state: State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateAccountRequest>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;
    let account = state
        .finance_manager_state
        .account_handler
        .create_account(*user.client_id(), request)
        .await?;

    Ok(Json(account))
}

async fn list_accounts(
    state: State<AppState>,
    headers: HeaderMap,
    Json(filters): Json<AccountListFilters>,
) -> HttpResult<impl IntoResponse> {
    let user = state.auth_state.auth_handler.authenticate(&headers).await?;
    let accounts = state
        .finance_manager_state
        .account_handler
        .list_accounts(*user.client_id(), filters)
        .await?;

    Ok(Json(accounts))
}
