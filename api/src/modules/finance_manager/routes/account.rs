use axum::{extract::State, response::IntoResponse, routing::post, Json, Router};
use http_error::HttpResult;

use crate::modules::{finance_manager::handler::account::CreateAccountRequest, routes::AppState};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest("/account", Router::new().route("/", post(create_account)))
}

async fn create_account(
    state: State<AppState>,
    Json(request): Json<CreateAccountRequest>,
) -> HttpResult<impl IntoResponse> {
    let account = state
        .finance_manager_state
        .account_handler
        .create_account(request)
        .await?;

    Ok(Json(account))
}
