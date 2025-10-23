use serde::{Deserialize, Serialize};

use super::telegram_update::TelegramMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub chat_id: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageResponse {
    pub ok: bool,
    pub result: TelegramMessage,
}
