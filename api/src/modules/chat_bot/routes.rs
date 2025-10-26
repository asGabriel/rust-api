use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use chrono::Utc;
use http_error::HttpResult;
use telegram_api::domain::{send_message::SendMessageRequest, telegram_update::TelegramUpdate};

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
    println!(
        "Message received from Telegram: {:?}, update_id: {}",
        payload.get_message(),
        payload.update_id
    );

    // TODO: receber a mensagem salvar o payload e retornar um ok rapidamente;
    // Implementar alguma mensageria para processar a mensagem em background;
    if let Some(message) = payload.get_message() {
        if let Some(text) = message.get_text() {
            match ChatCommand::from_message(text) {
                Ok(command) => {
                    println!("Comando recebido: {:?}", command);
                    state
                        .chat_bot_state
                        .chat_bot_handler
                        .handle_command(command, message.chat.id)
                        .await?;
                }
                Err(e) => {
                    println!(
                        "Erro ao processar comando: {}, chat_id: {} em {}",
                        e,
                        message.chat.id,
                        Utc::now().format("%Y-%m-%d %H:%M:%S")
                    );
                    state
                        .chat_bot_state
                        .telegram_gateway
                        .send_message(SendMessageRequest {
                            chat_id: message.chat.id,
                            text: format!("❌ {}", e.message),
                        })
                        .await?;
                }
            }
        }
    }

    Ok(StatusCode::OK)
}
