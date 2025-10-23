use reqwest::Client;

pub mod domain;
pub mod telegram_api;

pub struct TelegramApiClient {
    client: Client,
    host: String,
}

impl TelegramApiClient {
    pub fn new() -> Self {
        let url =
            std::env::var("TELEGRAM_API_URL").expect("Could not fetch telegram base url data.");
        let token =
            std::env::var("TELEGRAM_API_TOKEN").expect("Could not fetch telegram token data.");

        let host = format!("{}/bot{}", url, token);

        Self {
            client: Client::new(),
            host,
        }
    }
}
