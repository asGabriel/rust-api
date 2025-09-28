use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use http_error::HttpResult;

use crate::modules::{routes::AppState, telegram_bot::gateway::TelegramUpdate};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest("/webhook", {
        Router::new().route("/", post(handle_events))
    })
}

pub async fn handle_events(
    _state: State<AppState>,
    Json(payload): Json<TelegramUpdate>,
) -> HttpResult<impl IntoResponse> {
    println!("Telegram update: {:?}", payload);
    println!("Message: {:?}", payload.get_message());

    Ok(StatusCode::OK)
}
