use axum::{response::IntoResponse, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use http_error::HttpResult;
use serde::{Deserialize, Serialize};

use crate::modules::{
    chat_bot::{self},
    finance_manager::{self},
    AppState,
};

pub fn configure_services() -> Router<AppState> {
    let finance_manager_routes = finance_manager::configure_service_routes();
    let telegram_bot_routes = chat_bot::routes::configure_routes();

    Router::new().nest(
        "/api",
        Router::new()
            .merge(finance_manager_routes)
            .merge(telegram_bot_routes)
            .route("/status", get(api_status)),
    )
}

async fn api_status() -> HttpResult<impl IntoResponse> {
    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ApiStatusResponse {
        status_code: u16,
        message: String,
        timestamp: DateTime<Utc>,
        service: String,
    }

    let response = ApiStatusResponse {
        status_code: 200,
        message: "API is online and running".to_string(),
        timestamp: Utc::now(),
        service: "rust-api".to_string(),
    };

    Ok(Json(response))
}
