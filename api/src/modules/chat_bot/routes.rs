use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
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
    println!("Telegram update: {:?}", payload);
    println!("Message: {:?}", payload.get_message());

    // TODO: receber a mensagem salvar o payload e retornar um ok rapidamente;
    // Implementar alguma mensageria para processar a mensagem em background;
    if let Some(message) = payload.get_message() {
        if let Some(text) = message.get_text() {
            if let Some(command) = ChatCommand::from_message(text) {
                println!("Comando recebido: {:?}", command);
                state
                    .chat_bot_state
                    .chat_bot_handler
                    .handle_command(command, message.chat.id)
                    .await?;
            } else {
                println!("Mensagem não é um comando válido: {}", text);
                state
                    .chat_bot_state
                    .telegram_gateway
                    .send_message(SendMessageRequest {
                        chat_id: message.chat.id,
                        text: "Mensagem não é um comando válido".to_string(),
                    })
                    .await?;
            }
        }
    }

    Ok(StatusCode::OK)
}
