use axum::{response::IntoResponse, routing::get, Json, Router};
use chrono::{DateTime, Utc};
use http_error::HttpResult;
use serde::{Deserialize, Serialize};

use crate::modules::routes::AppState;

pub mod game_day;
pub mod played_match;
pub mod player;
pub mod team;

pub fn configure_routes() -> Router<AppState> {
    Router::new().route("/status", get(api_status))
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
        service: "matchmaking".to_string(),
    };

    Ok(Json(response))
}
