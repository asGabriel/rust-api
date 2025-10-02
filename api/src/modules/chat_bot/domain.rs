use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatCommandType {
    Debts,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCommand {
    pub command_type: ChatCommandType,
    pub raw_text: String,
    pub parameters: Vec<String>,
}

impl ChatCommand {
    /// Parse a message and extract the command.
    pub fn from_message(text: &str) -> Option<Self> {
        let text = text.trim();

        if !text.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = text[1..].split_whitespace().collect();
        if parts.is_empty() {
            return None;
        }

        let command_str = parts[0].to_lowercase();
        let parameters: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        let command_type = match command_str.as_str() {
            "debitos" | "debts" | "débitos" => ChatCommandType::Debts,
            _ => ChatCommandType::Unknown(command_str),
        };

        Some(ChatCommand {
            command_type,
            raw_text: text.to_string(),
            parameters,
        })
    }
}
