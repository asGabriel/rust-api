use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use http_error::HttpResult;
use telegram_api::domain::{send_message::SendMessageRequest, telegram_update::TelegramUpdate};
use tokio::task;

use crate::modules::{chat_bot::domain::ChatCommand, routes::AppState};

pub fn configure_routes() -> Router<AppState> {
    Router::new().nest("/webhook", {
        Router::new().route("/", post(handle_events))
    })
}

pub async fn handle_events(
    state: State<AppState>,
    Json(payload): Json<TelegramUpdate>,
) -> HttpResult<impl IntoResponse> {
    println!("Message received, update_id: {}", payload.update_id);

    let background_state = Arc::clone(&state.chat_bot_state);

    task::spawn(async move {
        if let Some(message) = payload.get_message() {
            if let Some(text) = message.get_text() {
                println!("Text: {}", text);
                match ChatCommand::from_message(text) {
                    Ok(command) => {
                        println!("Command: {:?}", command);
                        if let Err(err) = background_state
                            .chat_bot_handler
                            .handle_command(command, message.chat.id)
                            .await
                        {
                            eprintln!("Erro ao processar comando: {:?}", err);
                            let _ = background_state
                                .telegram_gateway
                                .send_message(SendMessageRequest {
                                    chat_id: message.chat.id,
                                    text: "❌ erro interno ao processar seu comando".to_string(),
                                })
                                .await;
                        }
                    }
                    Err(e) => {
                        let _ = background_state
                            .telegram_gateway
                            .send_message(SendMessageRequest {
                                chat_id: message.chat.id,
                                text: format!("❌ {}", e.message),
                            })
                            .await;
                    }
                }
            }
        }
    });

    Ok(StatusCode::OK)
}
