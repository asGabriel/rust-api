use serde::{Deserialize, Serialize};

/*
Docs:
https://core.telegram.org/bots/api#update
*/

/// Represents an incoming update from Telegram Bot API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramUpdate {
    pub update_id: u64,
    pub message: Option<TelegramMessage>,
    pub edited_message: Option<TelegramMessage>,
}

impl TelegramUpdate {
    pub fn get_message(&self) -> Option<&TelegramMessage> {
        self.message.as_ref()
    }

    pub fn get_edited_message(&self) -> Option<&TelegramMessage> {
        self.edited_message.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramMessage {
    pub message_id: u64,
    pub from: Option<TelegramUser>,
    pub date: u64,
    pub chat: TelegramChat,
    pub text: Option<String>,
}

impl TelegramMessage {
    pub fn get_text(&self) -> Option<&String> {
        self.text.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramUser {
    pub id: u64,
    pub is_bot: bool,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramChat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: String,
    pub title: Option<String>,
    pub username: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}
