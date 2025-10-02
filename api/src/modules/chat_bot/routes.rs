use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json, Router};
use http_error::HttpResult;
use telegram_api::TelegramUpdate;

use crate::modules::{
    chat_bot::domain::{ChatCommand, ChatCommandType},
    routes::AppState,
};

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

    // TODO: receber a mensagem salvar o payload e retornar um ok rapidamente;
    // Implementar alguma mensageria para processar a mensagem em background;
    if let Some(message) = payload.get_message() {
        if let Some(text) = message.get_text() {
            if let Some(command) = ChatCommand::from_message(text) {
                println!("Comando recebido: {:?}", command);

                match command.command_type {
                    ChatCommandType::Debts => {
                        // TODO: Implementar lógica de débitos
                    }
                    ChatCommandType::Unknown(cmd) => {
                        println!("Comando desconhecido: {}", cmd);
                        // TODO: Responder com erro de comando não encontrado
                    }
                }
            } else {
                println!("Mensagem não é um comando válido: {}", text);
            }
        }
    }

    Ok(StatusCode::OK)
}
